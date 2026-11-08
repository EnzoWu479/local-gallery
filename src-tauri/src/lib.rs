mod db;
mod media;
mod vault;

use std::sync::Mutex;
use std::time::Duration;

use db::{Folder, GalleryItem};
use rusqlite::Connection;
use serde::Serialize;
use tauri::{Manager, State};
use vault::Vault;

struct Db(Mutex<Connection>);

const PASSWORD_KEY: &str = "hidden_folders_password";

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

#[tauri::command]
async fn add_item(
    url: String,
    folder_id: Option<i64>,
    db: State<'_, Db>,
    vault: State<'_, Vault>,
) -> Result<GalleryItem, String> {
    let url = normalize_url(&url)?;
    guard_folder(&db.0.lock().unwrap(), &vault, folder_id)?;
    let probe_url = url.clone();
    let kind = blocking(move || media::detect_kind(&probe_url)).await?;
    db::insert(&db.0.lock().unwrap(), &url, kind, folder_id)
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
    let title = title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
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
fn delete_item(id: i64, db: State<Db>, vault: State<Vault>) -> Result<(), String> {
    let conn = db.0.lock().unwrap();
    guard_item(&conn, &vault, id)?;
    db::delete(&conn, id)
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
async fn unlock(password: String, db: State<'_, Db>, vault: State<'_, Vault>) -> Result<(), String> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("gallery.db"))?;
            app.manage(Db(Mutex::new(conn)));
            app.manage(Vault::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_items,
            add_item,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
