# ytdl — Educational YouTube Downloader in Rust

A command-line downloader built with Rust and the [`yt-dlp`](https://crates.io/crates/yt-dlp) crate.
On first run it auto-downloads the yt-dlp and ffmpeg executables into `./libs/`; subsequent
runs reuse them.

> **Responsible use:** Use this tool only on videos you own, videos under a Creative Commons
> licence, or other content you have explicit permission to download. Respect YouTube's
> [Terms of Service](https://www.youtube.com/t/terms).

---

## Prerequisites

Install the Rust toolchain (one-time):

```powershell
winget install Rustlang.Rustup
# then restart your terminal
```

No other manual installs are needed — yt-dlp and ffmpeg are fetched automatically.

---

## Build

```bash
cargo build --release
# binary: target/release/ytdl.exe  (Windows)
```

---

## Usage

```
ytdl <URL> [OPTIONS]

Arguments:
  <URL>   Video or playlist URL

Options:
  -o, --output <DIR>    Output directory        [default: downloads]
  -a, --audio           Audio-only, saved as MP3
  -q, --quality <Q>     best | high | medium | low | worst | <height e.g. 1080>
                        [default: best]
  -i, --info            Print metadata; do not download
      --libs <DIR>      yt-dlp / ffmpeg binary directory [default: libs]
  -h, --help            Print help
  -V, --version         Print version
```

### Examples

```bash
# Print video info only (no download)
cargo run -- --info "https://www.youtube.com/watch?v=VIDEO_ID"

# Download a video (best quality)
cargo run -- "https://www.youtube.com/watch?v=VIDEO_ID"

# Download at 720p
cargo run -- -q 720 "https://www.youtube.com/watch?v=VIDEO_ID"

# Extract audio as MP3
cargo run -- -a "https://www.youtube.com/watch?v=VIDEO_ID"

# Download an entire playlist
cargo run -- "https://www.youtube.com/playlist?list=PLAYLIST_ID"

# Save to a custom folder
cargo run -- -o ~/Videos "https://www.youtube.com/watch?v=VIDEO_ID"
```

---

## Project layout

```
src/
  main.rs   — entry point, tokio runtime, dispatching
  cli.rs    — clap argument definitions
  app.rs    — downloader setup, info, single-video, playlist logic
```

## Learning notes

| Rust concept | Where to look |
|---|---|
| Async / tokio runtime | `main.rs` — `#[tokio::main]` |
| Builder pattern | `app.rs` — `Downloader::with_new_binaries(...).await?.build().await?` |
| Error propagation | `app.rs` — `?` operator + `anyhow::Context` |
| CLI with derive | `cli.rs` — `#[derive(Parser)]` |
| Pattern matching | `app.rs` — `parse_quality()` |
| String sanitisation | `app.rs` — `sanitize()` |
