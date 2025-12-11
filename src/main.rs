use std::process::{Command, Stdio};
use clap::Parser;

/// Shrink an MP4 to a target size using ffmpeg re-encoding.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Input MP4 file path.
    input: String,
    /// Output MP4 file path.
    output: String,
    /// Target file size in bytes (default 10MB).
    #[arg(long, default_value_t = 10 * 1024 * 1024)]
    target_bytes: u64,
    /// Optional video bitrate (bps). If omitted, auto-calculated.
    #[arg(long)]
    video_bitrate: Option<u64>,
    /// Audio bitrate (bps).
    #[arg(long, default_value_t = 64_000)]
    audio_bitrate: u64,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Default video bitrate if not provided.
    let mut v_bitrate = args.video_bitrate.unwrap_or(500_000);

    // Probe duration; if available, back-calc bitrate to hit target size.
    let duration = probe_duration(&args.input).unwrap_or(0.0);
    if duration > 0.0 && args.video_bitrate.is_none() {
        // Leave 15% headroom and reserve audio.
        let reserve = (args.target_bytes as f64 * 0.85)
            - (args.audio_bitrate as f64 / 8.0 * duration);
        if reserve > 0.0 {
            let calc = (reserve * 8.0 / duration) as u64;
            v_bitrate = calc.clamp(200_000, 1_500_000);
        }
    }

    eprintln!(
        "duration={:.2}s, video_bitrate={}bps, audio_bitrate={}bps",
        duration, v_bitrate, args.audio_bitrate
    );

    // Encode with H.264/AAC; downscale width to <=640; high CRF for small size.
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            &args.input,
            "-c:v",
            "libx264",
            "-preset",
            "medium",
            "-b:v",
            &format!("{}k", v_bitrate / 1000),
            "-maxrate",
            &format!("{}k", v_bitrate / 1000),
            "-bufsize",
            &format!("{}k", v_bitrate / 500),
            "-vf",
            "scale='min(640,iw)':-2",
            "-c:a",
            "aac",
            "-b:a",
            &format!("{}k", args.audio_bitrate / 1000),
            "-movflags",
            "+faststart",
            "-crf",
            "32",
            &args.output,
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        eprintln!("ffmpeg failed, exit code: {:?}", status.code());
        std::process::exit(1);
    }

    Ok(())
}

/// Read video duration (seconds) via ffprobe.
fn probe_duration(path: &str) -> Option<f64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout)
        .trim()
        .to_string();
    s.parse::<f64>().ok()
}

