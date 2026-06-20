use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ytdl",
    about = "Educational YouTube downloader powered by yt-dlp and Rust",
    long_about = "Downloads YouTube videos using the yt-dlp library.\n\
                  yt-dlp and ffmpeg are auto-installed on first run into --libs.\n\
                  Use only on content you are permitted to download."
)]
pub struct Args {
    /// Video or playlist URL
    pub url: String,

    /// Output directory for downloaded files
    #[arg(short, long, default_value = "downloads")]
    pub output: PathBuf,

    /// Extract audio only and save as MP3
    #[arg(short, long)]
    pub audio: bool,

    /// Video quality: best | high | medium | low | worst | <height px, e.g. 1080>
    #[arg(short, long, default_value = "best")]
    pub quality: String,

    /// Print metadata and available formats; do not download
    #[arg(short, long)]
    pub info: bool,

    /// Directory where yt-dlp and ffmpeg binaries are stored (auto-downloaded on first run)
    #[arg(long, default_value = "libs")]
    pub libs: PathBuf,
}
