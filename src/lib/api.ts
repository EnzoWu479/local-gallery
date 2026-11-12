import { convertFileSrc, invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DownloadProgress, Folder, GalleryItem, UpdateInfo, UpdateProgress, VaultStatus } from "./types";

// Outside Tauri (plain `pnpm dev` in a browser) fall back to an in-memory mock,
// so the UI can be worked on without the Rust side.
const mocked = import.meta.env.DEV && !isTauri();

const invoke: typeof tauriInvoke = mocked
  ? async (cmd, args) => (await import("./api.mock")).mockInvoke(cmd, args)
  : tauriInvoke;

/**
 * Where the webview loads an item's media from: the `media://` protocol, which
 * reads only from the S3 store (see src-tauri/src/protocol.rs). The mock has no
 * store, so it uses the original link.
 */
export const mediaSrc = (item: GalleryItem) =>
  mocked ? item.url : convertFileSrc(String(item.id), "media");

// Thin typed wrappers over the Rust commands in src-tauri/src/lib.rs.

export const listItems = () => invoke<GalleryItem[]>("list_items");

/** Download progress for `progressId` arrives through `onDownloadProgress`. */
export const addItem = (url: string, folderId: number | null, progressId: string) =>
  invoke<GalleryItem>("add_item", { url, folderId, progressId });

export const storeItem = (id: number, progressId: string) =>
  invoke<GalleryItem>("store_item", { id, progressId });

/** Subscribes to download progress events. Resolves to an unsubscribe function. */
export const onDownloadProgress = async (handler: (progress: DownloadProgress) => void) =>
  mocked
    ? (await import("./api.mock")).onMockProgress(handler)
    : listen<DownloadProgress>("download-progress", (event) => handler(event.payload));

export const renameItem = (id: number, title: string | null) =>
  invoke<GalleryItem>("rename_item", { id, title });

export const deleteItem = (id: number) => invoke<void>("delete_item", { id });

export const moveItem = (id: number, folderId: number | null) =>
  invoke<GalleryItem>("move_item", { id, folderId });

export const listFolders = () => invoke<Folder[]>("list_folders");

export const createFolder = (name: string, hidden: boolean) =>
  invoke<Folder>("create_folder", { name, hidden });

export const updateFolder = (id: number, changes: { name?: string; hidden?: boolean }) =>
  invoke<Folder>("update_folder", { id, ...changes });

export const deleteFolder = (id: number) => invoke<void>("delete_folder", { id });

// Hidden folders password. The Rust side keeps the locked state and filters
// hidden content out of every response while locked.

export const vaultStatus = () => invoke<VaultStatus>("vault_status");

export const setPassword = (current: string | null, newPassword: string) =>
  invoke<void>("set_password", { current, newPassword });

export const unlock = (password: string) => invoke<void>("unlock", { password });

export const lock = () => invoke<void>("lock");

// Auto update (src-tauri/src/update.rs).

/** The newer version on GitHub, or null when this one is the latest. */
export const checkUpdate = () => invoke<UpdateInfo | null>("check_update");

/** Downloads and installs the version found by `checkUpdate`, then the app restarts. */
export const installUpdate = () => invoke<void>("install_update");

export const onUpdateProgress = async (handler: (progress: UpdateProgress) => void) =>
  mocked
    ? (await import("./api.mock")).onMockUpdateProgress(handler)
    : listen<UpdateProgress>("update-progress", (event) => handler(event.payload));
