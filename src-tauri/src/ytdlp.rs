//! Downloads from pages (YouTube, Vimeo, X…) with yt-dlp, the same way the
//! video_downloader_app does: best mp4 video + m4a audio, merged by ffmpeg.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::media::{Download, MAX_DOWNLOAD_BYTES, type_from_extension};

/// Finds a helper program: `LOCAL_GALLERY_<NAME>` env var, next to the app's
/// executable (installed builds), `src-tauri/binaries/<name>-*` (dev, filled
/// by `pnpm fetch-binaries`), then the PATH.
fn find_binary(name: &str) -> Option<PathBuf> {
    let env_var = format!("LOCAL_GALLERY_{}", name.to_uppercase().replace('-', ""));
    if let Ok(path) = std::env::var(env_var) {
        return Some(PathBuf::from(path));
    }

    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        let beside = dir.join(&file);
        if beside.exists() {
            return Some(beside);
        }
    }

    if cfg!(debug_assertions) {
        let bin_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
        let prefix = format!("{name}-");
        let found = std::fs::read_dir(bin_dir).ok().and_then(|entries| {
            entries.flatten().map(|e| e.path()).find(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(&prefix))
            })
        });
        if found.is_some() {
            return found;
        }
    }

    on_path(&file)
}

fn on_path(file: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(file))
        .find(|p| p.is_file())
}

fn command(path: &Path) -> Command {
    let mut cmd = Command::new(path);
    cmd.env("PYTHONUTF8", "1").env("PYTHONIOENCODING", "utf-8");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// yt-dlp format selection. With ffmpeg, separate video and audio streams are
/// merged into one mp4; without it only single-file formats work.
fn format(has_ffmpeg: bool) -> &'static str {
    if has_ffmpeg {
        "bv*[ext=mp4][height<=1080]+ba[ext=m4a]/b[ext=mp4][height<=1080]/bv*[height<=1080]+ba/b[height<=1080]/b"
    } else {
        "b[ext=mp4][height<=1080]/b[ext=mp4]/b"
    }
}

/// A `PROGRESS <downloaded> <total> <estimate>` line; yt-dlp writes `NA` for unknown values.
fn parse_progress(line: &str) -> Option<(u64, Option<u64>)> {
    let mut parts = line.strip_prefix("PROGRESS ")?.split_whitespace();
    let number = |s: Option<&str>| s.and_then(|v| v.parse::<f64>().ok()).map(|v| v as u64);
    let received = number(parts.next())?;
    let total = number(parts.next()).or(number(parts.next()));
    Some((received, total))
}

/// The most useful line of yt-dlp's stderr for the user.
fn error_message(stderr: &str) -> String {
    let line = stderr
        .lines()
        .rev()
        .find(|l| l.starts_with("ERROR:"))
        .or_else(|| stderr.lines().rev().find(|l| !l.trim().is_empty()))
        .unwrap_or("erro desconhecido")
        .trim_start_matches("ERROR:")
        .trim();
    if line.contains("Unsupported URL") {
        "Esse link não é uma imagem, áudio ou vídeo, e o site não é suportado pelo yt-dlp."
            .to_string()
    } else {
        format!("O yt-dlp não conseguiu baixar: {line}")
    }
}

pub struct PageDownload {
    pub download: Download,
    /// The page's title, used as the item title.
    pub title: Option<String>,
}

/// Downloads the media of a page into a temporary folder and reads it into
/// memory. Blocking — call it off the main thread.
pub fn download(
    url: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<PageDownload, String> {
    let ytdlp = find_binary("yt-dlp").ok_or(
        "Esse link é uma página, e para baixar dela é preciso o yt-dlp. \
         Instale com `winget install yt-dlp.yt-dlp` ou rode `pnpm fetch-binaries`.",
    )?;
    let ffmpeg = find_binary("ffmpeg");

    let dir = std::env::temp_dir().join(format!(
        "local-gallery-{}",
        crate::storage::random_hex(8)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let result = run(&ytdlp, ffmpeg.as_deref(), url, &dir, &mut on_progress);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn run(
    ytdlp: &Path,
    ffmpeg: Option<&Path>,
    url: &str,
    dir: &Path,
    on_progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<PageDownload, String> {
    let mut cmd = command(ytdlp);
    cmd.args(["--no-playlist", "--newline", "--progress", "--encoding", "utf-8"])
        .args(["--retries", "10", "--fragment-retries", "10"])
        .args(["--max-filesize", &MAX_DOWNLOAD_BYTES.to_string()])
        .args([
            "--progress-template",
            "download:PROGRESS %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s",
        ])
        .args(["--print", "after_move:FILE:%(filepath)s"])
        .args(["--print", "after_move:TITLE:%(title)s"])
        .args(["-f", format(ffmpeg.is_some())])
        .arg("-o")
        .arg(dir.join("media.%(ext)s"));
    if let Some(ffmpeg) = ffmpeg {
        cmd.args(["--merge-output-format", "mp4"])
            .arg("--ffmpeg-location")
            .arg(ffmpeg);
    }

    let mut child = cmd
        .arg("--")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Não foi possível iniciar o yt-dlp: {e}"))?;

    let mut stderr = child.stderr.take().unwrap();
    let stderr_thread = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });

    on_progress(0, None);
    let (mut file, mut title) = (None, None);
    for line in BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map_while(Result::ok)
    {
        if let Some((received, total)) = parse_progress(&line) {
            on_progress(received, total);
        } else if let Some(path) = line.strip_prefix("FILE:") {
            file = Some(PathBuf::from(path.trim()));
        } else if let Some(t) = line.strip_prefix("TITLE:") {
            title = Some(t.trim().to_string()).filter(|t| !t.is_empty() && t != "NA");
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let stderr = stderr_thread.join().unwrap_or_default();
    if !status.success() {
        return Err(error_message(&stderr));
    }

    // With --max-filesize, yt-dlp skips a big file and still exits with success.
    let file = file.filter(|f| f.exists()).ok_or_else(|| {
        format!(
            "O arquivo passa do limite de {} MB.",
            MAX_DOWNLOAD_BYTES / 1024 / 1024
        )
    })?;
    let (kind, content_type) = type_from_extension(&file.to_string_lossy())
        .ok_or("O yt-dlp baixou um formato que a galeria não sabe mostrar.")?;
    let bytes = std::fs::read(&file).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(format!(
            "O arquivo passa do limite de {} MB.",
            MAX_DOWNLOAD_BYTES / 1024 / 1024
        ));
    }

    Ok(PageDownload {
        download: Download {
            bytes,
            kind,
            content_type: content_type.to_string(),
        },
        title,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress_lines() {
        assert_eq!(parse_progress("PROGRESS 10 100 NA"), Some((10, Some(100))));
        assert_eq!(parse_progress("PROGRESS 10 NA 250.5"), Some((10, Some(250))));
        assert_eq!(parse_progress("PROGRESS 10 NA NA"), Some((10, None)));
        assert_eq!(parse_progress("PROGRESS NA NA NA"), None);
        assert_eq!(parse_progress("[download] 10%"), None);
    }

    #[test]
    fn explains_errors() {
        assert!(
            error_message("WARNING: x\nERROR: Unsupported URL: https://a.com")
                .contains("não é suportado")
        );
        assert_eq!(
            error_message("ERROR: [youtube] abc: Video unavailable\n"),
            "O yt-dlp não conseguiu baixar: [youtube] abc: Video unavailable"
        );
    }

    /// Needs network and yt-dlp: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn downloads_a_youtube_video() {
        let page = download("https://www.youtube.com/watch?v=jNQXAC9IVRw", |_, _| {}).unwrap();
        assert_eq!(page.download.kind, "video");
        assert_eq!(page.download.content_type, "video/mp4");
        assert!(page.download.bytes.len() > 10_000);
        assert!(page.title.is_some());
    }
}
