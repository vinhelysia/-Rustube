use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc,
};
use eframe::egui;
use crate::app;

// ── Quality enum ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Default)]
enum Quality {
    #[default]
    Best,
    P1080,
    P720,
    P480,
    Worst,
}

impl Quality {
    fn label(self) -> &'static str {
        match self {
            Self::Best  => "Best",
            Self::P1080 => "1080p",
            Self::P720  => "720p",
            Self::P480  => "480p",
            Self::Worst => "Worst",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Best  => "best",
            Self::P1080 => "1080",
            Self::P720  => "720",
            Self::P480  => "480",
            Self::Worst => "worst",
        }
    }
}

// ── App struct ────────────────────────────────────────────────────────────────

pub struct YtdlApp {
    url:          String,
    quality:      Quality,
    audio_only:   bool,
    playlist_all: bool,
    output_dir:   String,
    log:          Vec<String>,
    log_tx:       Sender<String>,
    log_rx:       Receiver<String>,
    downloading:  Arc<AtomicBool>,
}

impl YtdlApp {
    pub fn new() -> Self {
        let (log_tx, log_rx) = mpsc::channel();
        Self {
            url:          String::new(),
            quality:      Quality::default(),
            audio_only:   false,
            playlist_all: true,
            output_dir:   "downloads".to_string(),
            log:          Vec::new(),
            log_tx,
            log_rx,
            downloading:  Arc::new(AtomicBool::new(false)),
        }
    }

    fn start_download(&mut self, ctx: egui::Context) {
        let url          = self.url.trim().to_string();
        let quality      = self.quality.as_str().to_string();
        let audio_only   = self.audio_only;
        let playlist_all = self.playlist_all;
        let output_dir   = self.output_dir.clone();
        let tx           = self.log_tx.clone();
        let flag         = Arc::clone(&self.downloading);

        flag.store(true, Ordering::SeqCst);
        self.log.push(format!("——— {} ———", url));

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime");
            rt.block_on(async move {
                let result = run_download(url, quality, audio_only, playlist_all,
                                          output_dir, tx.clone()).await;
                match result {
                    Ok(_)  => tx.send("✓ Done.".into()).ok(),
                    Err(e) => tx.send(format!("✗ Error: {e}")).ok(),
                };
            });
            flag.store(false, Ordering::SeqCst);
            ctx.request_repaint();
        });
    }
}

const MAX_LOG_LINES: usize = 1000;

// ── eframe::App impl ─────────────────────────────────────────────────────────

impl eframe::App for YtdlApp {
    // Drain the log channel and schedule repaints here (has ctx access).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.log_rx.try_recv() {
            self.log.push(msg);
        }
        if self.log.len() > MAX_LOG_LINES {
            self.log.drain(0..self.log.len() - MAX_LOG_LINES);
        }
        if self.downloading.load(Ordering::SeqCst) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    // Render the window contents.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let is_downloading = self.downloading.load(Ordering::SeqCst);

        ui.heading("ytdl — YouTube Downloader");
        ui.add_space(8.0);

        // ── URL ───────────────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("URL:");
            ui.add(
                egui::TextEdit::singleline(&mut self.url)
                    .hint_text("https://www.youtube.com/watch?v=…")
                    .desired_width(f32::INFINITY),
            );
            if ui.button("✕").on_hover_text("Clear").clicked() {
                self.url.clear();
            }
        });

        ui.add_space(6.0);

        // ── Quality (hidden when audio-only) ──────────────────────────────────
        if !self.audio_only {
            ui.horizontal(|ui| {
                ui.label("Quality:");
                for q in [
                    Quality::Best,
                    Quality::P1080,
                    Quality::P720,
                    Quality::P480,
                    Quality::Worst,
                ] {
                    ui.radio_value(&mut self.quality, q, q.label());
                }
            });
        }

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.audio_only, "Audio only (MP3)");
            ui.checkbox(&mut self.playlist_all, "Download entire playlist");
        });

        ui.add_space(6.0);

        // ── Output directory ──────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Output:");
            ui.add(
                egui::TextEdit::singleline(&mut self.output_dir).desired_width(340.0),
            );
            if ui.button("Browse…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.output_dir = path.to_string_lossy().into_owned();
                }
            }
        });

        ui.add_space(10.0);

        // ── Action buttons ────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            let can_dl = !is_downloading && !self.url.trim().is_empty();
            let label  = if is_downloading { "⏳ Downloading…" } else { "⬇  Download" };
            if ui.add_enabled(can_dl, egui::Button::new(label)).clicked() {
                self.start_download(ui.ctx().clone());
            }
            if ui.button("🗑  Clear log").clicked() {
                self.log.clear();
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label(egui::RichText::new("Log").strong());
        ui.add_space(2.0);

        // ── Scrollable log area ───────────────────────────────────────────────
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(ui.available_height())
            .show(ui, |ui| {
                for line in &self.log {
                    ui.label(line);
                }
            });
    }
}

// ── Async download bridge ─────────────────────────────────────────────────────

async fn run_download(
    url:          String,
    quality:      String,
    audio_only:   bool,
    playlist_all: bool,
    output_dir:   String,
    tx:           Sender<String>,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(&output_dir)?;

    tx.send("Setting up yt-dlp / ffmpeg (auto-installs on first run)…".into()).ok();
    let downloader = app::ensure_downloader("libs".into(), output_dir.into()).await?;
    tx.send("Ready.".into()).ok();

    if app::is_playlist_url(&url) {
        if playlist_all {
            app::download_playlist(&downloader, &url, &tx).await
        } else {
            let stripped_url = app::strip_playlist_params(&url);
            if !app::has_video_reference(&stripped_url) {
                tx.send("This is a playlist-only URL — enable 'Download entire playlist' or paste a single video URL.".into()).ok();
                Ok(())
            } else {
                let vq = app::parse_quality(&quality);
                app::download_single(&downloader, &stripped_url, audio_only, vq, &tx).await
            }
        }
    } else {
        let vq = app::parse_quality(&quality);
        app::download_single(&downloader, &url, audio_only, vq, &tx).await
    }
}
