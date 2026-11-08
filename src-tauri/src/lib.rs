mod db;
mod media;
mod protocol;
mod storage;
mod vault;

use std::sync::Mutex;
use std::time::{Duration, Instant};

use db::{Folder, GalleryItem, StoredObject};
use rusqlite::Connection;
use serde::Serialize;
use storage::{Storage, StorageState};
use tauri::{AppHandle, Emitter, Manager, State};
use vault::Vault;

struct Db(Mutex<Connection>);

const PASSWORD_KEY: &str = "hidden_folders_password";
const S3_ACCESS_KEY: &str = "s3_access_key";
const S3_SECRET_KEY: &str = "s3_secret_key";

fn normalize_url(url: &str) -> Result<String, String> {
    let url = url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(url.to_string())
    } else {
        Err("O link precisa começar com http:// ou https://".to_string())
    }
}

fn normalize_folder_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        Err("Dê um nome para a pasta.".to_string())
    } else {
        Ok(name.to_string())
    }
}

// While locked, hidden folders and their items behave as if they did not exist.

fn guard_folder(conn: &Connection, vault: &Vault, id: Option<i64>) -> Result<(), String> {
    match id {
        Some(id) if db::folder_is_hidden(conn, id)? => vault.ensure_unlocked(),
        _ => Ok(()),
    }
}

fn guard_item(conn: &Connection, vault: &Vault, id: i64) -> Result<(), String> {
    if db::item_is_hidden(conn, id)? {
        vault.ensure_unlocked()
    } else {
        Ok(())
    }
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())
}

// Items

#[tauri::command]
fn list_items(db: State<Db>, vault: State<Vault>) -> Result<Vec<GalleryItem>, String> {
    db::list(&db.0.lock().unwrap(), vault.is_unlocked())
}

/// Downloads the link and puts it in the S3 store. Returns the media kind and
/// where it was stored.
/// A `download-progress` event. The frontend picks the `id` and passes it to
/// the command, then follows the events with that id.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress<'a> {
    id: &'a str,
    /// "download" while bytes arrive, then "save" while the copy goes to S3.
    phase: &'static str,
    received: u64,
    /// The file size, when the site sends a `Content-Length`.
    total: Option<u64>,
}

fn emit_progress(
    app: &AppHandle,
    id: Option<&str>,
    phase: &'static str,
    received: u64,
    total: Option<u64>,
) {
    if let Some(id) = id {
        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                id,
                phase,
                received,
                total,
            },
        );
    }
}

/// Downloads the link and puts it in the S3 store, reporting progress under
/// `progress_id`. Returns the media kind and where it was stored.
async fn download_to_storage(
    url: String,
    storage: &Storage,
    app: &AppHandle,
    progress_id: Option<String>,
) -> Result<(&'static str, StoredObject), String> {
    let reporter = app.clone();
    let id = progress_id.clone();
    let download = blocking(move || {
        // At most ten events a second, plus the last one.
        let mut last: Option<Instant> = None;
        media::download(&url, |received, total| {
            let finished = total == Some(received);
            if finished || last.is_none_or(|t| t.elapsed() >= Duration::from_millis(100)) {
                last = Some(Instant::now());
                emit_progress(&reporter, id.as_deref(), "download", received, total);
            }
        })
    })
    .await??;

    let size = download.bytes.len() as u64;
    emit_progress(app, progress_id.as_deref(), "save", size, Some(size));
    let object = StoredObject {
        key: storage::new_object_key(),
        content_type: download.content_type,
        size,
    };
    storage
        .put(&object.key, &download.bytes, &object.content_type)
        .await?;
    Ok((download.kind, object))
}

/// Adds a link: downloads it, stores the copy in S3, then saves the item.
/// Nothing is saved if any step fails.
#[tauri::command]
async fn add_item(
    url: String,
    folder_id: Option<i64>,
    progress_id: Option<String>,
    app: AppHandle,
    db: State<'_, Db>,
    vault: State<'_, Vault>,
    storage: State<'_, StorageState>,
) -> Result<GalleryItem, String> {
    let url = normalize_url(&url)?;
    let storage = storage.get()?;
    {
        let conn = db.0.lock().unwrap();
        guard_folder(&conn, &vault, folder_id)?;
        if db::url_exists(&conn, &url)? {
            return Err("Esse link já está na galeria.".to_string());
        }
    }

    let (kind, object) = download_to_storage(url.clone(), storage, &app, progress_id).await?;
    let inserted = db::insert(&db.0.lock().unwrap(), &url, kind, folder_id, Some(&object));
    if inserted.is_err() {
        let _ = storage.delete(&object.key).await;
    }
    inserted
}

/// Downloads and stores the media of an item saved before the S3 store existed.
#[tauri::command]
async fn store_item(
    id: i64,
    progress_id: Option<String>,
    app: AppHandle,
    db: State<'_, Db>,
    vault: State<'_, Vault>,
    storage: State<'_, StorageState>,
) -> Result<GalleryItem, String> {
    let storage = storage.get()?;
    let url = {
        let conn = db.0.lock().unwrap();
        guard_item(&conn, &vault, id)?;
        let item = db::get(&conn, id)?.ok_or_else(|| "Item não encontrado.".to_string())?;
        if item.stored {
            return Ok(item);
        }
        item.url
    };

    let (kind, object) = download_to_storage(url, storage, &app, progress_id).await?;
    let updated = db::set_object(&db.0.lock().unwrap(), id, kind, &object);
    if updated.is_err() {
        let _ = storage.delete(&object.key).await;
    }
    updated
}

#[tauri::command]
fn rename_item(
    id: i64,
    title: Option<String>,
    db: State<Db>,
    vault: State<Vault>,
) -> Result<GalleryItem, String> {
    let conn = db.0.lock().unwrap();
    guard_item(&conn, &vault, id)?;
    let title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    db::set_title(&conn, id, title.as_deref())
}

#[tauri::command]
fn move_item(
    id: i64,
    folder_id: Option<i64>,
    db: State<Db>,
    vault: State<Vault>,
) -> Result<GalleryItem, String> {
    let conn = db.0.lock().unwrap();
    guard_item(&conn, &vault, id)?;
    guard_folder(&conn, &vault, folder_id)?;
    db::set_folder(&conn, id, folder_id)
}

#[tauri::command]
async fn delete_item(
    id: i64,
    db: State<'_, Db>,
    vault: State<'_, Vault>,
    storage: State<'_, StorageState>,
) -> Result<(), String> {
    let object = {
        let conn = db.0.lock().unwrap();
        guard_item(&conn, &vault, id)?;
        let object = db::get_object(&conn, id)?;
        db::delete(&conn, id)?;
        object
    };

    if let (Some(object), Ok(storage)) = (object, storage.get()) {
        if let Err(e) = storage.delete(&object.key).await {
            eprintln!("could not delete {} from storage: {e}", object.key);
        }
    }
    Ok(())
}

// Folders

#[tauri::command]
fn list_folders(db: State<Db>, vault: State<Vault>) -> Result<Vec<Folder>, String> {
    db::list_folders(&db.0.lock().unwrap(), vault.is_unlocked())
}

#[tauri::command]
fn create_folder(
    name: String,
    hidden: bool,
    db: State<Db>,
    vault: State<Vault>,
) -> Result<Folder, String> {
    let name = normalize_folder_name(&name)?;
    if hidden {
        vault.ensure_unlocked()?;
    }
    db::insert_folder(&db.0.lock().unwrap(), &name, hidden)
}

#[tauri::command]
fn update_folder(
    id: i64,
    name: Option<String>,
    hidden: Option<bool>,
    db: State<Db>,
    vault: State<Vault>,
) -> Result<Folder, String> {
    let name = name.as_deref().map(normalize_folder_name).transpose()?;
    let conn = db.0.lock().unwrap();
    guard_folder(&conn, &vault, Some(id))?;
    if hidden == Some(true) {
        vault.ensure_unlocked()?;
    }
    db::update_folder(&conn, id, name.as_deref(), hidden)
}

#[tauri::command]
fn delete_folder(id: i64, db: State<Db>, vault: State<Vault>) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    guard_folder(&conn, &vault, Some(id))?;
    db::delete_folder(&conn, id)
}

// Hidden folders password

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VaultStatus {
    has_password: bool,
    unlocked: bool,
}

#[tauri::command]
fn vault_status(db: State<Db>, vault: State<Vault>) -> Result<VaultStatus, String> {
    let has_password = db::get_setting(&db.0.lock().unwrap(), PASSWORD_KEY)?.is_some();
    Ok(VaultStatus {
        has_password,
        unlocked: vault.is_unlocked(),
    })
}

/// Creates the password, or changes it when `current` matches the stored one.
/// Either way the hidden folders end up unlocked.
#[tauri::command]
async fn set_password(
    current: Option<String>,
    new_password: String,
    db: State<'_, Db>,
    vault: State<'_, Vault>,
) -> Result<(), String> {
    if new_password.chars().count() < vault::MIN_PASSWORD_LEN {
        return Err(format!(
            "A senha precisa ter pelo menos {} caracteres.",
            vault::MIN_PASSWORD_LEN
        ));
    }

    let stored = db::get_setting(&db.0.lock().unwrap(), PASSWORD_KEY)?;
    if let Some(stored) = stored {
        let current = current.unwrap_or_default();
        if !blocking(move || vault::verify(&current, &stored)).await? {
            return Err("A senha atual está incorreta.".to_string());
        }
    }

    let hash = blocking(move || vault::hash(&new_password)).await??;
    db::set_setting(&db.0.lock().unwrap(), PASSWORD_KEY, &hash)?;
    vault.set_unlocked(true);
    Ok(())
}

#[tauri::command]
async fn unlock(
    password: String,
    db: State<'_, Db>,
    vault: State<'_, Vault>,
) -> Result<(), String> {
    let stored = db::get_setting(&db.0.lock().unwrap(), PASSWORD_KEY)?
        .ok_or_else(|| "Crie uma senha primeiro.".to_string())?;

    let valid = blocking(move || {
        let valid = vault::verify(&password, &stored);
        if !valid {
            // Slow down guessing.
            std::thread::sleep(Duration::from_millis(800));
        }
        valid
    })
    .await?;

    if !valid {
        return Err("Senha incorreta.".to_string());
    }
    vault.set_unlocked(true);
    Ok(())
}

#[tauri::command]
fn lock(vault: State<Vault>) {
    vault.set_unlocked(false);
}

/// The S3 credentials for the local rclone server, created on first use.
fn s3_credentials(conn: &Connection) -> Result<(String, String), String> {
    let access = db::get_setting(conn, S3_ACCESS_KEY)?;
    let secret = db::get_setting(conn, S3_SECRET_KEY)?;
    if let (Some(access), Some(secret)) = (access, secret) {
        return Ok((access, secret));
    }
    let (access, secret) = (storage::random_hex(10), storage::random_hex(20));
    db::set_setting(conn, S3_ACCESS_KEY, &access)?;
    db::set_setting(conn, S3_SECRET_KEY, &secret)?;
    Ok((access, secret))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("media", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                responder.respond(protocol::serve(&app, &request).await);
            });
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("gallery.db"))?;

            let storage = s3_credentials(&conn)
                .and_then(|(access, secret)| Storage::start(&dir.join("s3"), &access, &secret));
            if let Err(e) = &storage {
                eprintln!("media storage unavailable: {e}");
            }

            app.manage(Db(Mutex::new(conn)));
            app.manage(Vault::default());
            app.manage(StorageState(storage));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_items,
            add_item,
            store_item,
            rename_item,
            delete_item,
            move_item,
            list_folders,
            create_folder,
            update_folder,
            delete_folder,
            vault_status,
            set_password,
            unlock,
            lock
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Ok(storage) = app.state::<StorageState>().get() {
                    storage.stop();
                }
            }
        });
}
