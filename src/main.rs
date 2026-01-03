/*
 * MP4视频压缩工具
 * 
 * Copyright (c) 2024
 * 
 * 本程序使用FFmpeg进行视频压缩，FFmpeg遵循LGPL/GPL许可证。
 * FFmpeg版权归FFmpeg开发者所有，详见 https://ffmpeg.org/legal.html
 * 
 * 本程序采用MIT许可证发布。
 * 
 * 功能：
 * - 使用两遍编码技术精确控制输出文件大小
 * - 支持动画和实拍视频的优化压缩
 * - 自动嵌入FFmpeg工具，无需单独安装
 */

use std::process::{Command, Stdio};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use clap::Parser;

// 嵌入FFmpeg和FFprobe可执行文件
const FFMPEG_BYTES: &[u8] = include_bytes!("resource/ffmpeg.exe");
const FFPROBE_BYTES: &[u8] = include_bytes!("resource/ffprobe.exe");

/// 使用两遍编码压缩MP4到目标大小，保持更好的画质
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// 输入MP4文件路径
    input: String,
    /// 输出MP4文件路径
    output: String,
    /// 目标文件大小（字节，默认10MB）
    #[arg(long, default_value_t = 10 * 1024 * 1024)]
    target_bytes: u64,
    /// 视频内容类型：animation（动画）或 film（实拍视频）
    #[arg(long, default_value = "animation")]
    content_type: String,
    /// 音频码率（bps）
    #[arg(long, default_value_t = 64_000)]
    audio_bitrate: u64,
}

fn main() -> std::io::Result<()> {
    // 显示版权信息
    eprintln!("MP4视频压缩工具 v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("Copyright (c) 2024");
    eprintln!("本程序使用FFmpeg，FFmpeg遵循LGPL/GPL许可证");
    eprintln!();

    let args = Args::parse();

    // 提取并准备FFmpeg工具
    let (ffmpeg_path, ffprobe_path) = extract_ffmpeg_tools()?;

    // 获取视频信息
    let duration = probe_duration(&args.input, &ffprobe_path).unwrap_or(0.0);
    if duration <= 0.0 {
        eprintln!("错误: 无法获取视频时长");
        std::process::exit(1);
    }

    // 计算精确的视频码率（两遍编码需要精确码率）
    // 预留5%的容器开销和音频空间
    let audio_size = (args.audio_bitrate as f64 / 8.0) * duration;
    let container_overhead = args.target_bytes as f64 * 0.05;
    let available_for_video = args.target_bytes as f64 - audio_size - container_overhead;
    
    if available_for_video <= 0.0 {
        eprintln!("错误: 目标文件太小，无法容纳音频");
        std::process::exit(1);
    }

    let v_bitrate = ((available_for_video * 8.0) / duration) as u64;
    
    eprintln!(
        "视频时长: {:.2}秒, 目标大小: {:.2}MB",
        duration,
        args.target_bytes as f64 / (1024.0 * 1024.0)
    );
    eprintln!(
        "视频码率: {}kbps, 音频码率: {}kbps",
        v_bitrate / 1000,
        args.audio_bitrate / 1000
    );

    // 根据内容类型选择tune参数
    let tune = if args.content_type == "animation" {
        "animation"
    } else {
        "film"
    };

    // 创建临时日志文件用于两遍编码
    let log_file = Path::new(&args.output)
        .parent()
        .unwrap_or(Path::new("."))
        .join("ffmpeg2pass.log");

    // 准备两遍编码共用的视频参数（必须完全一致）
    let v_bitrate_k = v_bitrate / 1000;
    let maxrate_k = (v_bitrate * 110 / 100) / 1000;  // 允许10%波动
    let bufsize_k = (v_bitrate * 2) / 1000;  // 更大的缓冲区
    let scale_filter = "scale='min(1280,iw)':-2";

    // 第一遍编码：分析视频
    eprintln!("第一遍编码（分析视频）...");
    let mut pass1_cmd = Command::new(&ffmpeg_path);
    pass1_cmd
        .args([
            "-y",
            "-i", &args.input,
            "-c:v", "libx264",
            "-preset", "slow",
            "-tune", tune,  // 必须与第二遍一致
            "-b:v", &format!("{}k", v_bitrate_k),
            "-maxrate", &format!("{}k", maxrate_k),  // 必须与第二遍一致
            "-bufsize", &format!("{}k", bufsize_k),  // 必须与第二遍一致
            "-pass", "1",
            "-passlogfile", log_file.to_str().unwrap(),
            "-vf", scale_filter,
            "-an",  // 第一遍不编码音频
            "-f", "null",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    
    // Windows上使用NUL，Unix上使用/dev/null
    #[cfg(windows)]
    pass1_cmd.arg("NUL");
    #[cfg(not(windows))]
    pass1_cmd.arg("/dev/null");
    
    let pass1_status = pass1_cmd.status()?;

    if !pass1_status.success() {
        eprintln!("第一遍编码失败");
        std::process::exit(1);
    }

    // 第二遍编码：实际编码（视频参数必须与第一遍完全一致）
    eprintln!("第二遍编码（实际压缩）...");
    let pass2_status = Command::new(&ffmpeg_path)
        .args([
            "-y",
            "-i", &args.input,
            "-c:v", "libx264",
            "-preset", "slow",
            "-tune", tune,
            "-b:v", &format!("{}k", v_bitrate_k),
            "-maxrate", &format!("{}k", maxrate_k),
            "-bufsize", &format!("{}k", bufsize_k),
            "-pass", "2",
            "-passlogfile", log_file.to_str().unwrap(),
            "-vf", scale_filter,
            "-c:a", "aac",
            "-b:a", &format!("{}k", args.audio_bitrate / 1000),
            "-movflags", "+faststart",
            &args.output,
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    // 清理临时文件
    let _ = std::fs::remove_file(&log_file);
    let _ = std::fs::remove_file(log_file.with_extension("log.mbtree"));

    if !pass2_status.success() {
        eprintln!("第二遍编码失败");
        std::process::exit(1);
    }

    eprintln!("压缩完成！");
    Ok(())
}

/// 提取FFmpeg工具到临时目录
fn extract_ffmpeg_tools() -> std::io::Result<(PathBuf, PathBuf)> {
    // 获取临时目录
    let temp_dir = std::env::temp_dir().join("mp4_shrink_ffmpeg");
    
    // 创建临时目录（如果不存在）
    fs::create_dir_all(&temp_dir)?;
    
    let ffmpeg_path = temp_dir.join("ffmpeg.exe");
    let ffprobe_path = temp_dir.join("ffprobe.exe");
    
    // 如果文件已存在且大小正确，则跳过提取（避免重复提取）
    let need_extract_ffmpeg = !ffmpeg_path.exists() || 
        fs::metadata(&ffmpeg_path)?.len() != FFMPEG_BYTES.len() as u64;
    let need_extract_ffprobe = !ffprobe_path.exists() || 
        fs::metadata(&ffprobe_path)?.len() != FFPROBE_BYTES.len() as u64;
    
    if need_extract_ffmpeg {
        let mut file = fs::File::create(&ffmpeg_path)?;
        file.write_all(FFMPEG_BYTES)?;
        file.sync_all()?;
    }
    
    if need_extract_ffprobe {
        let mut file = fs::File::create(&ffprobe_path)?;
        file.write_all(FFPROBE_BYTES)?;
        file.sync_all()?;
    }
    
    Ok((ffmpeg_path, ffprobe_path))
}

/// 通过ffprobe读取视频时长（秒）
fn probe_duration(path: &str, ffprobe_path: &Path) -> Option<f64> {
    // 检查文件是否存在
    if !Path::new(path).exists() {
        eprintln!("错误: 文件不存在: {}", path);
        return None;
    }

    let out = match Command::new(ffprobe_path)
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
    {
        Ok(output) => output,
        Err(e) => {
            eprintln!("错误: 无法执行ffprobe: {}", e);
            return None;
        }
    };

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        eprintln!("ffprobe执行失败: {}", stderr);
        return None;
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let trimmed = stdout.trim();
    
    if trimmed.is_empty() {
        eprintln!("错误: ffprobe未返回时长信息");
        return None;
    }

    match trimmed.parse::<f64>() {
        Ok(duration) if duration > 0.0 => Some(duration),
        Ok(_) => {
            eprintln!("错误: 解析到的时长为0或负数");
            None
        }
        Err(e) => {
            eprintln!("错误: 无法解析时长 '{}': {}", trimmed, e);
            None
        }
    }
}

