import { invoke as tauriInvoke, isTauri } from "@tauri-apps/api/core";
import type { Folder, GalleryItem, VaultStatus } from "./types";

// Outside Tauri (plain `pnpm dev` in a browser) fall back to an in-memory mock,
// so the UI can be worked on without the Rust side.
const invoke: typeof tauriInvoke =
  import.meta.env.DEV && !isTauri()
    ? async (cmd, args) => (await import("./api.mock")).mockInvoke(cmd, args)
    : tauriInvoke;

// Thin typed wrappers over the Rust commands in src-tauri/src/lib.rs.

export const listItems = () => invoke<GalleryItem[]>("list_items");

export const addItem = (url: string, folderId: number | null) =>
  invoke<GalleryItem>("add_item", { url, folderId });

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
