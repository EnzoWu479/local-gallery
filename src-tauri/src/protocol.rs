//! The `media://` protocol. The webview loads every image, audio and video
//! from here (`http://media.localhost/<item id>` on Windows); the bytes come
//! only from the S3 store, never from the original link.

use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager};

use crate::storage::StorageState;
use crate::vault::Vault;
use crate::{Db, db};

/// Most bytes sent per response. Media elements ask for `bytes=0-` and then
/// keep requesting the next range, so large videos never load whole.
const CHUNK: u64 = 4 * 1024 * 1024;

/// Parses a `Range` header into an inclusive byte range within `size`,
/// at most `CHUNK` long. `Err` means the range is unsatisfiable (416).
/// `Ok(None)` means there is no usable range and the whole file is sent.
fn parse_range(value: &str, size: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(spec) = value.trim().strip_prefix("bytes=") else {
        return Ok(None);
    };
    // Multiple ranges are not supported; serve the first.
    let spec = spec.split(',').next().unwrap_or("").trim();
    let Some((start, end)) = spec.split_once('-') else {
        return Ok(None);
    };
    if size == 0 {
        return Err(());
    }

    let (start, end) = match (start.trim(), end.trim()) {
        // bytes=-500: the last 500 bytes
        ("", suffix) => {
            let Ok(n) = suffix.parse::<u64>() else {
                return Ok(None);
            };
            if n == 0 {
                return Err(());
            }
            (size.saturating_sub(n), size - 1)
        }
        (start, end) => {
            let Ok(start) = start.parse::<u64>() else {
                return Ok(None);
            };
            let end = if end.is_empty() {
                size - 1
            } else {
                let Ok(end) = end.parse::<u64>() else {
                    return Ok(None);
                };
                end.min(size - 1)
            };
            (start, end)
        }
    };

    if start >= size || start > end {
        return Err(());
    }
    Ok(Some((start, end.min(start + CHUNK - 1))))
}

/// Headers every response carries. The page and `media.localhost` are
/// different origins, so CORS headers let the page read statuses and ranges.
fn response() -> tauri::http::response::Builder {
    Response::builder()
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            "Content-Range, Accept-Ranges",
        )
        // Not cached: an item can move into a hidden folder, and a cached copy
        // would stay viewable after locking. The store is local, so it is cheap.
        .header(header::CACHE_CONTROL, "no-store")
}

fn empty(status: StatusCode) -> Response<Vec<u8>> {
    response().status(status).body(Vec::new()).unwrap()
}

pub async fn serve(app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Ok(id) = request.uri().path().trim_matches('/').parse::<i64>() else {
        return empty(StatusCode::BAD_REQUEST);
    };

    let object = {
        let db = app.state::<Db>();
        let conn = db.0.lock().unwrap();
        let hidden = db::item_is_hidden(&conn, id).unwrap_or(true);
        if hidden && !app.state::<Vault>().is_unlocked() {
            return empty(StatusCode::FORBIDDEN);
        }
        match db::get_object(&conn, id) {
            Ok(Some(object)) => object,
            Ok(None) => return empty(StatusCode::NOT_FOUND),
            Err(_) => return empty(StatusCode::INTERNAL_SERVER_ERROR),
        }
    };

    let state = app.state::<StorageState>();
    let Ok(storage) = state.get() else {
        return empty(StatusCode::SERVICE_UNAVAILABLE);
    };

    let range = match request
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
    {
        Some(value) => match parse_range(value, object.size) {
            Ok(range) => range,
            Err(()) => {
                return response()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{}", object.size))
                    .body(Vec::new())
                    .unwrap();
            }
        },
        None => None,
    };

    let bytes = match storage.get(&object.key, range).await {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("could not read {} from storage: {e}", object.key);
            return empty(StatusCode::BAD_GATEWAY);
        }
    };

    let builder = response()
        .header(header::CONTENT_TYPE, &object.content_type)
        .header(header::ACCEPT_RANGES, "bytes");
    let builder = match range {
        Some((start, end)) => builder.status(StatusCode::PARTIAL_CONTENT).header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{}", object.size),
        ),
        None => builder.status(StatusCode::OK),
    };
    builder.body(bytes).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1024 * 1024;

    #[test]
    fn parses_closed_and_open_ranges() {
        assert_eq!(parse_range("bytes=0-99", 1000), Ok(Some((0, 99))));
        assert_eq!(parse_range("bytes=900-", 1000), Ok(Some((900, 999))));
        assert_eq!(parse_range("bytes=990-5000", 1000), Ok(Some((990, 999))));
        assert_eq!(parse_range("bytes=-100", 1000), Ok(Some((900, 999))));
        assert_eq!(parse_range("bytes=-5000", 1000), Ok(Some((0, 999))));
    }

    #[test]
    fn caps_each_response_at_one_chunk() {
        assert_eq!(parse_range("bytes=0-", 100 * MB), Ok(Some((0, CHUNK - 1))));
        assert_eq!(
            parse_range("bytes=10-", 100 * MB),
            Ok(Some((10, 10 + CHUNK - 1)))
        );
    }

    #[test]
    fn rejects_unsatisfiable_ranges() {
        assert_eq!(parse_range("bytes=1000-", 1000), Err(()));
        assert_eq!(parse_range("bytes=50-10", 1000), Err(()));
        assert_eq!(parse_range("bytes=-0", 1000), Err(()));
    }

    #[test]
    fn ignores_ranges_it_cannot_read() {
        assert_eq!(parse_range("items=0-10", 1000), Ok(None));
        assert_eq!(parse_range("bytes=abc-", 1000), Ok(None));
        assert_eq!(parse_range("bytes=0-10,20-30", 1000), Ok(Some((0, 10))));
    }
}
