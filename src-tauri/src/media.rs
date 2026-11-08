use std::io::Read;
use std::time::Duration;

/// Largest file the gallery will download.
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

pub struct Download {
    pub bytes: Vec<u8>,
    pub kind: &'static str,
    pub content_type: String,
}

/// Media kind and MIME type from the URL path extension, ignoring query and fragment.
pub fn type_from_extension(url: &str) -> Option<(&'static str, &'static str)> {
    let path = url.split(['?', '#']).next()?;
    let last_segment = path.rsplit('/').next()?;
    let (_, ext) = last_segment.rsplit_once('.')?;

    let found = match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "png" => ("image", "image/png"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "avif" => ("image", "image/avif"),
        "bmp" => ("image", "image/bmp"),
        "svg" => ("image", "image/svg+xml"),
        "ico" => ("image", "image/x-icon"),
        "mp3" => ("audio", "audio/mpeg"),
        "wav" => ("audio", "audio/wav"),
        "ogg" | "oga" => ("audio", "audio/ogg"),
        "flac" => ("audio", "audio/flac"),
        "m4a" => ("audio", "audio/mp4"),
        "aac" => ("audio", "audio/aac"),
        "opus" => ("audio", "audio/opus"),
        "weba" => ("audio", "audio/webm"),
        "mp4" | "m4v" => ("video", "video/mp4"),
        "webm" => ("video", "video/webm"),
        "mov" => ("video", "video/quicktime"),
        "ogv" => ("video", "video/ogg"),
        _ => return None,
    };
    Some(found)
}

fn kind_from_mime(mime: &str) -> Option<&'static str> {
    if mime.starts_with("image/") {
        Some("image")
    } else if mime.starts_with("audio/") {
        Some("audio")
    } else if mime.starts_with("video/") {
        Some("video")
    } else {
        None
    }
}

/// Decide what was downloaded. A media `Content-Type` wins; a generic one
/// (missing or octet-stream) falls back to the URL extension; anything else,
/// such as an HTML page, is not media.
fn classify(url: &str, content_type: Option<&str>) -> Option<(&'static str, String)> {
    let mime = content_type
        .and_then(|ct| ct.split(';').next())
        .map(|m| m.trim().to_ascii_lowercase())
        .unwrap_or_default();

    if let Some(kind) = kind_from_mime(&mime) {
        return Some((kind, mime));
    }
    let generic =
        mime.is_empty() || mime == "application/octet-stream" || mime == "binary/octet-stream";
    if generic {
        return type_from_extension(url).map(|(kind, mime)| (kind, mime.to_string()));
    }
    None
}

fn too_big() -> String {
    format!(
        "O arquivo passa do limite de {} MB.",
        MAX_DOWNLOAD_BYTES / 1024 / 1024
    )
}

/// What a link turned out to be.
pub enum Fetched {
    /// A direct image, audio or video file.
    Media(Download),
    /// Something else, usually a web page; yt-dlp may find media in it.
    Page,
}

/// Download a link if it is an image, audio or video.
/// `on_progress(received, total)` is called as bytes arrive; `total` is the
/// `Content-Length`, when the site sends one. Blocking — call it off the main thread.
pub fn download(
    url: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Fetched, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(Duration::from_secs(600)))
        .build()
        .into();

    let mut response = agent.get(url).call().map_err(|e| match e {
        ureq::Error::StatusCode(code) => format!("O site recusou o download (HTTP {code})."),
        ureq::Error::Timeout(_) => "O download demorou demais.".to_string(),
        other => format!("Não foi possível baixar o link: {other}"),
    })?;

    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    let content_type = header("content-type");
    let total = header("content-length").and_then(|v| v.parse::<u64>().ok());

    let Some((kind, content_type)) = classify(url, content_type.as_deref()) else {
        return Ok(Fetched::Page);
    };
    if total.is_some_and(|t| t > MAX_DOWNLOAD_BYTES) {
        return Err(too_big());
    }

    let mut reader = response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_BYTES)
        .reader();
    let mut bytes = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut chunk = vec![0u8; 64 * 1024];
    on_progress(0, total);
    loop {
        let read = reader.read(&mut chunk).map_err(|e| {
            if bytes.len() as u64 >= MAX_DOWNLOAD_BYTES {
                too_big()
            } else {
                format!("O download falhou: {e}")
            }
        })?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        on_progress(bytes.len() as u64, total);
    }

    Ok(Fetched::Media(Download {
        bytes,
        kind,
        content_type,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types_by_extension() {
        assert_eq!(
            type_from_extension("https://a.com/x/photo.JPG"),
            Some(("image", "image/jpeg"))
        );
        assert_eq!(
            type_from_extension("https://a.com/song.mp3?dl=1#t=3"),
            Some(("audio", "audio/mpeg"))
        );
        assert_eq!(
            type_from_extension("https://a.com/clip.webm"),
            Some(("video", "video/webm"))
        );
        assert_eq!(type_from_extension("https://a.com"), None);
        assert_eq!(type_from_extension("https://a.com/page"), None);
        assert_eq!(type_from_extension("https://a.com/file.html"), None);
    }

    #[test]
    fn media_content_type_wins() {
        assert_eq!(
            classify("https://a.com/x", Some("video/mp4")),
            Some(("video", "video/mp4".into()))
        );
        assert_eq!(
            classify("https://a.com/x.png", Some("audio/mpeg; charset=binary")),
            Some(("audio", "audio/mpeg".into()))
        );
    }

    #[test]
    fn generic_content_type_falls_back_to_extension() {
        assert_eq!(
            classify("https://a.com/x.mp4", Some("application/octet-stream")),
            Some(("video", "video/mp4".into()))
        );
        assert_eq!(
            classify("https://a.com/x.jpg", None),
            Some(("image", "image/jpeg".into()))
        );
        assert_eq!(
            classify("https://a.com/x", Some("application/octet-stream")),
            None
        );
    }

    #[test]
    fn rejects_pages_even_with_a_media_extension() {
        assert_eq!(
            classify("https://a.com/x.jpg", Some("text/html; charset=utf-8")),
            None
        );
    }

    fn media(fetched: Result<Fetched, String>) -> Download {
        match fetched.unwrap() {
            Fetched::Media(download) => download,
            Fetched::Page => panic!("expected media"),
        }
    }

    #[test]
    fn a_web_page_is_handed_to_yt_dlp() {
        let head =
            "HTTP/1.1 200 OK
Content-Type: text/html
Connection: close

".to_string();
        let url = serve_once(head, b"<html></html>".to_vec());
        assert!(matches!(download(&url, |_, _| {}), Ok(Fetched::Page)));
    }

    /// Serves one HTTP response on a local port and returns its URL.
    fn serve_once(head: String, body: Vec<u8>) -> String {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/clip.mp4", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0u8; 1024];
            let _ = socket.read(&mut request);
            socket.write_all(head.as_bytes()).unwrap();
            socket.write_all(&body).unwrap();
        });
        url
    }

    #[test]
    fn reports_progress_up_to_the_content_length() {
        let body: Vec<u8> = (0..=255u8).cycle().take(300_000).collect();
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let url = serve_once(head, body.clone());

        let mut calls = Vec::new();
        let download = media(download(&url, |received, total| calls.push((received, total))));

        assert_eq!(download.bytes, body);
        assert_eq!(download.kind, "video");
        let total = Some(body.len() as u64);
        assert_eq!(calls.first(), Some(&(0, total)));
        assert_eq!(calls.last(), Some(&(body.len() as u64, total)));
        assert!(calls.windows(2).all(|w| w[0].0 <= w[1].0));
    }

    #[test]
    fn reports_bytes_without_a_total_when_the_size_is_unknown() {
        let body = vec![7u8; 100_000];
        let head =
            "HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nConnection: close\r\n\r\n".to_string();
        let url = serve_once(head, body.clone());

        let mut last = None;
        let download = media(download(&url, |received, total| last = Some((received, total))));

        assert_eq!(download.bytes.len(), body.len());
        assert_eq!(last, Some((body.len() as u64, None)));
    }
}
