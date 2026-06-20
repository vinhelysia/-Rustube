use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use yt_dlp::{
    model::selector::{AudioCodecPreference, AudioQuality, VideoQuality},
    Downloader,
};

// ── Setup ──────────────────────────────────────────────────────────────────

pub async fn ensure_downloader(libs_dir: PathBuf, output_dir: PathBuf) -> Result<Downloader> {
    let downloader = Downloader::with_new_binaries(libs_dir, &output_dir)
        .await
        .context("Failed to install yt-dlp / ffmpeg")?
        .build()
        .await
        .context("Failed to build downloader")?;
    Ok(downloader)
}

// ── Single video ────────────────────────────────────────────────────────────

pub async fn download_single(
    downloader: &Downloader,
    url: &str,
    audio: bool,
    quality: VideoQuality,
    tx: &Sender<String>,
) -> Result<()> {
    tx.send("Fetching video info…".into()).ok();
    let video = downloader
        .fetch_video_infos(url)
        .await
        .context("Failed to fetch video info")?;

    tx.send(format!("Title:  {}", video.title)).ok();
    tx.send(format!(
        "Channel: {}",
        video
            .channel
            .as_deref()
            .or(video.uploader.as_deref())
            .unwrap_or("(unknown)")
    ))
    .ok();
    tx.send(format!(
        "Duration: {}",
        video.duration_string.as_deref().unwrap_or("(unknown)")
    ))
    .ok();

    let stem = sanitize(&video.title);

    if audio {
        let filename = format!("{}.mp3", stem);
        tx.send(format!("Downloading audio → {}", filename)).ok();
        let path = downloader
            .download_audio_stream_with_quality(
                &video,
                &filename,
                AudioQuality::Best,
                AudioCodecPreference::MP3,
            )
            .await
            .context("Audio download failed")?;
        tx.send(format!("Saved: {}", path.display())).ok();
    } else {
        let filename = format!("{}.mp4", stem);
        tx.send(format!("Downloading video → {}", filename)).ok();
        let path = downloader
            .download(&video, &filename)
            .video_quality(quality)
            .audio_quality(AudioQuality::Best)
            .execute()
            .await
            .context("Video download failed")?;
        tx.send(format!("Saved: {}", path.display())).ok();
    }

    Ok(())
}

// ── Playlist ────────────────────────────────────────────────────────────────

pub async fn download_playlist(
    downloader: &Downloader,
    url: &str,
    tx: &Sender<String>,
) -> Result<()> {
    tx.send("Fetching playlist info…".into()).ok();
    let playlist = downloader
        .fetch_playlist_infos(url)
        .await
        .context("Failed to fetch playlist info")?;

    let total = playlist.entries.len();
    tx.send(format!(
        "Playlist: {} ({} videos) — downloading all…",
        playlist.title, total
    ))
    .ok();

    let results = downloader
        .download_playlist(&playlist, "%(title)s.%(ext)s")
        .await
        .context("Playlist download failed")?;

    tx.send(format!("Done: {}/{} videos.", results.len(), total)).ok();
    for path in &results {
        tx.send(format!("  ↳ {}", path.display())).ok();
    }

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────────────

pub fn parse_quality(q: &str) -> VideoQuality {
    match q.to_lowercase().as_str() {
        "best" => VideoQuality::Best,
        "high" => VideoQuality::High,
        "medium" => VideoQuality::Medium,
        "low" => VideoQuality::Low,
        "worst" => VideoQuality::Worst,
        other => {
            if let Ok(h) = other.parse::<u32>() {
                VideoQuality::CustomHeight(h)
            } else {
                VideoQuality::Best
            }
        }
    }
}

pub fn is_playlist_url(url: &str) -> bool {
    url.contains("list=") || url.contains("/playlist")
}

pub fn strip_playlist_params(url: &str) -> String {
    let (main_part, fragment) = match url.split_once('#') {
        Some((m, f)) => (m, Some(f)),
        None => (url, None),
    };

    let (base, query) = match main_part.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (main_part, None),
    };

    let mut result = base.to_string();

    if let Some(q) = query {
        let mut kept_pairs = Vec::new();
        for pair in q.split('&') {
            if pair.is_empty() {
                continue;
            }
            let key = match pair.split_once('=') {
                Some((k, _)) => k,
                None => pair,
            };
            if key != "list" && key != "index" && key != "start_radio" {
                kept_pairs.push(pair);
            }
        }
        if !kept_pairs.is_empty() {
            result.push('?');
            result.push_str(&kept_pairs.join("&"));
        }
    }

    if let Some(f) = fragment {
        result.push('#');
        result.push_str(f);
    }

    result
}

pub fn has_video_reference(url: &str) -> bool {
    if url.contains("v=") {
        return true;
    }
    if let Some(pos) = url.find("youtu.be/") {
        let after = &url[pos + 9..];
        let first_seg = after.split(&['?', '#', '/'][..]).next().unwrap_or("");
        if !first_seg.is_empty() && first_seg != "playlist" {
            return true;
        }
    }
    false
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_playlist_params() {
        assert_eq!(
            strip_playlist_params("https://www.youtube.com/watch?v=X&list=Y"),
            "https://www.youtube.com/watch?v=X"
        );
        assert_eq!(
            strip_playlist_params("https://www.youtube.com/watch?v=X&list=Y&index=2&start_radio=1"),
            "https://www.youtube.com/watch?v=X"
        );
        assert_eq!(
            strip_playlist_params("youtu.be/X?list=Y"),
            "youtu.be/X"
        );
        assert_eq!(
            strip_playlist_params("youtu.be/X?list=Y&index=2"),
            "youtu.be/X"
        );
        assert_eq!(
            strip_playlist_params("https://www.youtube.com/watch?list=Y&v=X"),
            "https://www.youtube.com/watch?v=X"
        );
        assert_eq!(
            strip_playlist_params("https://www.youtube.com/playlist?list=PL123"),
            "https://www.youtube.com/playlist"
        );
    }

    #[test]
    fn test_has_video_reference() {
        assert!(has_video_reference("https://www.youtube.com/watch?v=X"));
        assert!(has_video_reference("youtu.be/X"));
        assert!(has_video_reference("https://youtu.be/abc_123"));
        assert!(!has_video_reference("https://www.youtube.com/playlist"));
        assert!(!has_video_reference("https://youtu.be/playlist"));
        assert!(!has_video_reference("https://youtu.be/"));
    }
}


