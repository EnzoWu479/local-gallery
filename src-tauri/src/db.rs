use std::path::Path;

use rusqlite::{Connection, ErrorCode, OptionalExtension, Row, params};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryItem {
    pub id: i64,
    pub url: String,
    pub kind: String,
    pub title: Option<String>,
    pub folder_id: Option<i64>,
    pub created_at: String,
    /// Whether a copy of the media is in the S3 store.
    pub stored: bool,
}

impl GalleryItem {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            url: row.get("url")?,
            kind: row.get("kind")?,
            title: row.get("title")?,
            folder_id: row.get("folder_id")?,
            created_at: row.get("created_at")?,
            stored: row.get("stored")?,
        })
    }
}

/// Where an item's media lives in the S3 store. Never sent to the frontend.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredObject {
    pub key: String,
    pub content_type: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: i64,
    pub name: String,
    pub hidden: bool,
    pub created_at: String,
}

impl Folder {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            hidden: row.get("hidden")?,
            created_at: row.get("created_at")?,
        })
    }
}

/// Schema steps, applied in order and tracked with `PRAGMA user_version`.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS items (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        url        TEXT NOT NULL UNIQUE,
        kind       TEXT NOT NULL,
        title      TEXT,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );",
    "CREATE TABLE folders (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        name       TEXT NOT NULL UNIQUE,
        hidden     INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
    ALTER TABLE items ADD COLUMN folder_id INTEGER REFERENCES folders(id) ON DELETE SET NULL;",
    "CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
    "ALTER TABLE items ADD COLUMN object_key TEXT;
    ALTER TABLE items ADD COLUMN content_type TEXT;
    ALTER TABLE items ADD COLUMN size INTEGER;",
];

const ITEM_COLUMNS: &str =
    "id, url, kind, title, folder_id, created_at, object_key IS NOT NULL AS stored";
const FOLDER_COLUMNS: &str = "id, name, hidden, created_at";

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    for (step, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        // Each step and its version bump commit together.
        conn.execute_batch(&format!(
            "BEGIN; {sql} PRAGMA user_version = {}; COMMIT;",
            step + 1
        ))?;
    }
    Ok(())
}

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    migrate(&conn)?;
    Ok(conn)
}

fn unique_violation(e: rusqlite::Error, message: &str) -> String {
    match e.sqlite_error_code() {
        Some(ErrorCode::ConstraintViolation) => message.to_string(),
        _ => e.to_string(),
    }
}

// Items

/// Items, newest first. Without `include_hidden`, items in hidden folders are left out.
pub fn list(conn: &Connection, include_hidden: bool) -> Result<Vec<GalleryItem>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {ITEM_COLUMNS} FROM items
             WHERE ?1 OR folder_id IS NULL OR folder_id NOT IN (SELECT id FROM folders WHERE hidden = 1)
             ORDER BY id DESC"
        ))
        .map_err(|e| e.to_string())?;
    let items = stmt
        .query_map(params![include_hidden], GalleryItem::from_row)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(items)
}

pub fn url_exists(conn: &Connection, url: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM items WHERE url = ?1)",
        params![url],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn insert(
    conn: &Connection,
    url: &str,
    kind: &str,
    folder_id: Option<i64>,
    object: Option<&StoredObject>,
) -> Result<GalleryItem, String> {
    conn.execute(
        "INSERT INTO items (url, kind, folder_id, object_key, content_type, size)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            url,
            kind,
            folder_id,
            object.map(|o| &o.key),
            object.map(|o| &o.content_type),
            object.map(|o| o.size as i64),
        ],
    )
    .map_err(|e| unique_violation(e, "Esse link já está na galeria."))?;
    get(conn, conn.last_insert_rowid())?.ok_or_else(|| "Item não encontrado.".to_string())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<GalleryItem>, String> {
    conn.query_row(
        &format!("SELECT {ITEM_COLUMNS} FROM items WHERE id = ?1"),
        params![id],
        GalleryItem::from_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn set_title(conn: &Connection, id: i64, title: Option<&str>) -> Result<GalleryItem, String> {
    conn.execute(
        "UPDATE items SET title = ?1 WHERE id = ?2",
        params![title, id],
    )
    .map_err(|e| e.to_string())?;
    get(conn, id)?.ok_or_else(|| "Item não encontrado.".to_string())
}

pub fn get_object(conn: &Connection, id: i64) -> Result<Option<StoredObject>, String> {
    conn.query_row(
        "SELECT object_key, content_type, size FROM items WHERE id = ?1 AND object_key IS NOT NULL",
        params![id],
        |row| {
            Ok(StoredObject {
                key: row.get(0)?,
                content_type: row.get(1)?,
                size: row.get::<_, i64>(2)? as u64,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Records the stored copy of an item that had none (and its detected kind).
pub fn set_object(
    conn: &Connection,
    id: i64,
    kind: &str,
    object: &StoredObject,
) -> Result<GalleryItem, String> {
    conn.execute(
        "UPDATE items SET kind = ?1, object_key = ?2, content_type = ?3, size = ?4 WHERE id = ?5",
        params![
            kind,
            object.key,
            object.content_type,
            object.size as i64,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    get(conn, id)?.ok_or_else(|| "Item não encontrado.".to_string())
}

pub fn set_folder(
    conn: &Connection,
    id: i64,
    folder_id: Option<i64>,
) -> Result<GalleryItem, String> {
    conn.execute(
        "UPDATE items SET folder_id = ?1 WHERE id = ?2",
        params![folder_id, id],
    )
    .map_err(|e| e.to_string())?;
    get(conn, id)?.ok_or_else(|| "Item não encontrado.".to_string())
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM items WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Folders

const DUPLICATE_FOLDER: &str = "Já existe uma pasta com esse nome.";

pub fn list_folders(conn: &Connection, include_hidden: bool) -> Result<Vec<Folder>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {FOLDER_COLUMNS} FROM folders WHERE ?1 OR hidden = 0 ORDER BY name COLLATE NOCASE"
        ))
        .map_err(|e| e.to_string())?;
    let folders = stmt
        .query_map(params![include_hidden], Folder::from_row)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(folders)
}

pub fn get_folder(conn: &Connection, id: i64) -> Result<Folder, String> {
    conn.query_row(
        &format!("SELECT {FOLDER_COLUMNS} FROM folders WHERE id = ?1"),
        params![id],
        Folder::from_row,
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Pasta não encontrada.".to_string())
}

pub fn insert_folder(conn: &Connection, name: &str, hidden: bool) -> Result<Folder, String> {
    conn.execute(
        "INSERT INTO folders (name, hidden) VALUES (?1, ?2)",
        params![name, hidden],
    )
    .map_err(|e| unique_violation(e, DUPLICATE_FOLDER))?;
    get_folder(conn, conn.last_insert_rowid())
}

pub fn update_folder(
    conn: &Connection,
    id: i64,
    name: Option<&str>,
    hidden: Option<bool>,
) -> Result<Folder, String> {
    conn.execute(
        "UPDATE folders SET name = COALESCE(?1, name), hidden = COALESCE(?2, hidden) WHERE id = ?3",
        params![name, hidden, id],
    )
    .map_err(|e| unique_violation(e, DUPLICATE_FOLDER))?;
    get_folder(conn, id)
}

/// Deletes the folder; its items stay in the gallery without a folder.
pub fn delete_folder(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM folders WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn folder_is_hidden(conn: &Connection, id: i64) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM folders WHERE id = ?1 AND hidden = 1)",
        params![id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn item_is_hidden(conn: &Connection, id: i64) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM items JOIN folders ON folders.id = items.folder_id
            WHERE items.id = ?1 AND folders.hidden = 1
        )",
        params![id],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

// Settings

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn insert_list_rename_delete() {
        let conn = memory_db();
        let a = insert(&conn, "https://a.com/1.png", "image", None, None).unwrap();
        let b = insert(&conn, "https://a.com/2.mp3", "audio", None, None).unwrap();

        let items = list(&conn, true).unwrap();
        assert_eq!(
            items.iter().map(|i| i.id).collect::<Vec<_>>(),
            vec![b.id, a.id]
        );

        let renamed = set_title(&conn, a.id, Some("Foto")).unwrap();
        assert_eq!(renamed.title.as_deref(), Some("Foto"));

        delete(&conn, a.id).unwrap();
        assert_eq!(list(&conn, true).unwrap().len(), 1);
    }

    #[test]
    fn rejects_duplicate_url() {
        let conn = memory_db();
        insert(&conn, "https://a.com/1.png", "image", None, None).unwrap();
        let err = insert(&conn, "https://a.com/1.png", "image", None, None).unwrap_err();
        assert_eq!(err, "Esse link já está na galeria.");
    }

    #[test]
    fn folders_hold_items_and_release_them_on_delete() {
        let conn = memory_db();
        let folder = insert_folder(&conn, "Viagens", true).unwrap();
        assert!(folder.hidden);

        let item = insert(&conn, "https://a.com/1.png", "image", Some(folder.id), None).unwrap();
        assert_eq!(item.folder_id, Some(folder.id));

        let shown = update_folder(&conn, folder.id, None, Some(false)).unwrap();
        assert!(!shown.hidden);
        assert_eq!(shown.name, "Viagens");

        delete_folder(&conn, folder.id).unwrap();
        assert_eq!(get(&conn, item.id).unwrap().unwrap().folder_id, None);
    }

    #[test]
    fn moves_items_between_folders() {
        let conn = memory_db();
        let folder = insert_folder(&conn, "Músicas", false).unwrap();
        let item = insert(&conn, "https://a.com/2.mp3", "audio", None, None).unwrap();
        assert_eq!(
            set_folder(&conn, item.id, Some(folder.id))
                .unwrap()
                .folder_id,
            Some(folder.id)
        );
        assert_eq!(set_folder(&conn, item.id, None).unwrap().folder_id, None);
    }

    #[test]
    fn leaves_hidden_folders_and_their_items_out_unless_asked() {
        let conn = memory_db();
        let open = insert_folder(&conn, "Abertas", false).unwrap();
        let secret = insert_folder(&conn, "Segredos", true).unwrap();
        let loose = insert(&conn, "https://a.com/1.png", "image", None, None).unwrap();
        let shown = insert(&conn, "https://a.com/2.png", "image", Some(open.id), None).unwrap();
        let hidden = insert(&conn, "https://a.com/3.png", "image", Some(secret.id), None).unwrap();

        let ids = |items: Vec<GalleryItem>| items.into_iter().map(|i| i.id).collect::<Vec<_>>();
        assert_eq!(ids(list(&conn, false).unwrap()), vec![shown.id, loose.id]);
        assert_eq!(
            ids(list(&conn, true).unwrap()),
            vec![hidden.id, shown.id, loose.id]
        );
        assert_eq!(list_folders(&conn, false).unwrap().len(), 1);
        assert_eq!(list_folders(&conn, true).unwrap().len(), 2);

        assert!(item_is_hidden(&conn, hidden.id).unwrap());
        assert!(!item_is_hidden(&conn, shown.id).unwrap());
        assert!(!item_is_hidden(&conn, loose.id).unwrap());
        assert!(folder_is_hidden(&conn, secret.id).unwrap());
        assert!(!folder_is_hidden(&conn, open.id).unwrap());
    }

    #[test]
    fn tracks_the_stored_copy() {
        let conn = memory_db();
        let object = StoredObject {
            key: "media/abc".into(),
            content_type: "video/mp4".into(),
            size: 42,
        };
        let stored = insert(&conn, "https://a.com/v", "video", None, Some(&object)).unwrap();
        assert!(stored.stored);
        assert_eq!(get_object(&conn, stored.id).unwrap(), Some(object.clone()));
        assert!(url_exists(&conn, "https://a.com/v").unwrap());

        let legacy = insert(&conn, "https://a.com/old.png", "image", None, None).unwrap();
        assert!(!legacy.stored);
        assert_eq!(get_object(&conn, legacy.id).unwrap(), None);

        let fixed = set_object(&conn, legacy.id, "image", &object).unwrap();
        assert!(fixed.stored);
    }

    #[test]
    fn stores_settings() {
        let conn = memory_db();
        assert_eq!(get_setting(&conn, "k").unwrap(), None);
        set_setting(&conn, "k", "a").unwrap();
        set_setting(&conn, "k", "b").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap().as_deref(), Some("b"));
    }

    #[test]
    fn rejects_duplicate_folder_name() {
        let conn = memory_db();
        insert_folder(&conn, "Viagens", false).unwrap();
        assert_eq!(
            insert_folder(&conn, "Viagens", true).unwrap_err(),
            DUPLICATE_FOLDER
        );
    }

    #[test]
    fn migrates_a_version_zero_database() {
        // A database created before folders existed: items table, user_version 0.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.execute(
            "INSERT INTO items (url, kind) VALUES ('https://a.com/x.png', 'image')",
            [],
        )
        .unwrap();

        migrate(&conn).unwrap();
        let items = list(&conn, true).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].folder_id, None);
    }
}
