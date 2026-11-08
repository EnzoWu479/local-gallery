use std::fs::File;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use s3::creds::Credentials;
use s3::{Bucket, Region};

const BUCKET: &str = "gallery";
const READY_TIMEOUT: Duration = Duration::from_secs(10);

/// The media store: a local `rclone serve s3` process owned by the app, and a
/// client for its single bucket. The server only listens on 127.0.0.1.
pub struct Storage {
    bucket: Box<Bucket>,
    server: Mutex<Option<Child>>,
}

/// Managed state. Holds the reason when storage could not start (e.g. rclone
/// is not installed), so the gallery still opens and commands can explain it.
pub struct StorageState(pub Result<Storage, String>);

impl StorageState {
    pub fn get(&self) -> Result<&Storage, String> {
        self.0
            .as_ref()
            .map_err(|e| format!("O armazenamento de mídias não está disponível: {e}"))
    }
}

fn rclone_binary() -> String {
    std::env::var("LOCAL_GALLERY_RCLONE").unwrap_or_else(|_| "rclone".to_string())
}

fn free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener
        .local_addr()
        .map(|a| a.port())
        .map_err(|e| e.to_string())
}

impl Storage {
    /// Starts rclone serving `root` (each subfolder is a bucket) and waits until
    /// it accepts connections. rclone's log goes to `<root>/../rclone.log`.
    pub fn start(root: &Path, access_key: &str, secret_key: &str) -> Result<Self, String> {
        std::fs::create_dir_all(root.join(BUCKET)).map_err(|e| e.to_string())?;
        let log_path = root.with_file_name("rclone.log");
        let log = File::create(&log_path).map_err(|e| e.to_string())?;

        let port = free_port()?;
        let addr = format!("127.0.0.1:{port}");

        let mut command = Command::new(rclone_binary());
        command
            .args(["serve", "s3", "--auth-key"])
            .arg(format!("{access_key},{secret_key}"))
            .args(["--addr", &addr])
            .arg(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "rclone não encontrado. Instale com `winget install Rclone.Rclone`.".to_string()
            }
            _ => format!("não foi possível iniciar o rclone: {e}"),
        })?;

        let started = Instant::now();
        loop {
            if TcpStream::connect(&addr).is_ok() {
                break;
            }
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!(
                    "o rclone fechou ao iniciar ({status}). Veja {}",
                    log_path.display()
                ));
            }
            if started.elapsed() > READY_TIMEOUT {
                let _ = child.kill();
                return Err("o rclone demorou demais para iniciar.".to_string());
            }
            thread::sleep(Duration::from_millis(50));
        }

        let region = Region::Custom {
            region: "us-east-1".to_string(),
            endpoint: format!("http://{addr}"),
        };
        let credentials = Credentials::new(Some(access_key), Some(secret_key), None, None, None)
            .map_err(|e| e.to_string())?;
        let bucket = Bucket::new(BUCKET, region, credentials)
            .map_err(|e| e.to_string())?
            .with_path_style();

        Ok(Self {
            bucket,
            server: Mutex::new(Some(child)),
        })
    }

    pub fn stop(&self) {
        if let Some(mut child) = self.server.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub async fn put(&self, key: &str, bytes: &[u8], content_type: &str) -> Result<(), String> {
        let response = self
            .bucket
            .put_object_with_content_type(key, bytes, content_type)
            .await
            .map_err(|e| e.to_string())?;
        check_status(response.status_code(), "salvar a mídia")
    }

    /// Reads the whole object, or the inclusive byte range `start..=end`.
    pub async fn get(&self, key: &str, range: Option<(u64, u64)>) -> Result<Vec<u8>, String> {
        let response = match range {
            Some((start, end)) => self.bucket.get_object_range(key, start, Some(end)).await,
            None => self.bucket.get_object(key).await,
        }
        .map_err(|e| e.to_string())?;
        check_status(response.status_code(), "ler a mídia")?;
        Ok(response.bytes().to_vec())
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        let response = self
            .bucket
            .delete_object(key)
            .await
            .map_err(|e| e.to_string())?;
        check_status(response.status_code(), "apagar a mídia")
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        self.stop();
    }
}

fn check_status(status: u16, action: &str) -> Result<(), String> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(format!(
            "Não foi possível {action} (S3 respondeu {status})."
        ))
    }
}

/// Random lowercase hex, `bytes * 2` characters long.
pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).expect("system random source unavailable");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn new_object_key() -> String {
    format!("media/{}", random_hex(16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_keys_are_random_hex() {
        let a = new_object_key();
        let b = new_object_key();
        assert_ne!(a, b);
        assert!(a.starts_with("media/"));
        assert_eq!(a.len(), "media/".len() + 32);
        assert!(a["media/".len()..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Needs rclone on PATH: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn round_trips_through_real_rclone() {
        let dir = std::env::temp_dir().join(format!("local-gallery-test-{}", random_hex(4)));
        let storage = Storage::start(&dir.join("s3"), "key", "secret").unwrap();

        tauri::async_runtime::block_on(async {
            let body: Vec<u8> = (0..=255u8).cycle().take(10_000).collect();
            storage.put("media/test", &body, "video/mp4").await.unwrap();
            assert_eq!(storage.get("media/test", None).await.unwrap(), body);
            assert_eq!(
                storage.get("media/test", Some((10, 19))).await.unwrap(),
                body[10..=19]
            );
            storage.delete("media/test").await.unwrap();
            assert!(storage.get("media/test", None).await.is_err());
        });

        storage.stop();
        let _ = std::fs::remove_dir_all(dir);
    }
}
