//! Auto update from the GitHub releases (see .github/workflows/release.yml).
//! The endpoint and the public key that checks the signature are in
//! tauri.conf.json, under `plugins.updater`.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// The update found by the last check, kept for `install_update`.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    current_version: String,
    notes: Option<String>,
}

/// An `update-progress` event while the installer downloads.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    received: u64,
    total: Option<u64>,
}

/// Looks for a newer version. `None` when this one is the latest.
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<Option<UpdateInfo>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("Não foi possível procurar atualizações: {e}"))?;

    let info = update.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
    });
    *pending.0.lock().unwrap() = update;
    Ok(info)
}

/// Downloads the update found by `check_update`, checks its signature and runs
/// the installer. On Windows the installer closes the app and opens the new
/// version; elsewhere the app restarts itself.
#[tauri::command]
pub async fn install_update(app: AppHandle, pending: State<'_, PendingUpdate>) -> Result<(), String> {
    let update = pending
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| "Nenhuma atualização pendente.".to_string())?;

    let mut received = 0u64;
    let mut last: Option<Instant> = None;
    let reporter = app.clone();
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let finished = total == Some(received);
                if finished || last.is_none_or(|t| t.elapsed() >= Duration::from_millis(100)) {
                    last = Some(Instant::now());
                    let _ = reporter.emit("update-progress", UpdateProgress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(|e| format!("Falha ao baixar a atualização: {e}"))?;

    // The installer replaces the executable, so nothing of ours may keep
    // running: rclone is stopped before the process exits.
    crate::stop_storage(&app);
    update
        .install(bytes)
        .map_err(|e| format!("Falha ao instalar a atualização: {e}. Feche e abra o app de novo."))?;
    app.restart();
}
