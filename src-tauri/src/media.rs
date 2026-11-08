use std::time::Duration;

/// Guess the media kind from the URL path extension, ignoring query and fragment.
pub fn kind_from_extension(url: &str) -> Option<&'static str> {
    let path = url.split(['?', '#']).next()?;
    let last_segment = path.rsplit('/').next()?;
    let (_, ext) = last_segment.rsplit_once('.')?;

    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" | "bmp" | "svg" | "ico" => Some("image"),
        "mp3" | "wav" | "ogg" | "oga" | "flac" | "m4a" | "aac" | "opus" | "weba" => Some("audio"),
        "mp4" | "webm" | "mov" | "m4v" | "ogv" => Some("video"),
        _ => None,
    }
}

fn kind_from_content_type(content_type: &str) -> &'static str {
    let mime = content_type.split(';').next().unwrap_or("").trim();
    if mime.starts_with("image/") {
        "image"
    } else if mime.starts_with("audio/") {
        "audio"
    } else if mime.starts_with("video/") {
        "video"
    } else {
        "unknown"
    }
}

/// Detect the media kind of a URL: by extension first, then by a HEAD request.
/// Blocking — call it off the main thread.
pub fn detect_kind(url: &str) -> &'static str {
    if let Some(kind) = kind_from_extension(url) {
        return kind;
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();

    match agent.head(url).call() {
        Ok(response) => response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(kind_from_content_type)
            .unwrap_or("unknown"),
        Err(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_extension() {
        assert_eq!(kind_from_extension("https://a.com/x/photo.JPG"), Some("image"));
        assert_eq!(kind_from_extension("https://a.com/song.mp3?dl=1#t=3"), Some("audio"));
        assert_eq!(kind_from_extension("https://a.com/clip.webm"), Some("video"));
        assert_eq!(kind_from_extension("https://a.com"), None);
        assert_eq!(kind_from_extension("https://a.com/page"), None);
        assert_eq!(kind_from_extension("https://a.com/file.html"), None);
    }

    #[test]
    fn detects_by_content_type() {
        assert_eq!(kind_from_content_type("image/png"), "image");
        assert_eq!(kind_from_content_type("audio/mpeg; charset=binary"), "audio");
        assert_eq!(kind_from_content_type("text/html"), "unknown");
    }
}
