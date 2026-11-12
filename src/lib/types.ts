export type MediaKind = "image" | "audio" | "video" | "unknown";

export interface GalleryItem {
  id: number;
  url: string;
  kind: MediaKind;
  title: string | null;
  folderId: number | null;
  createdAt: string;
  /** Whether a copy of the media is in the local S3 store. */
  stored: boolean;
}

export interface Folder {
  id: number;
  name: string;
  hidden: boolean;
  createdAt: string;
}

export interface VaultStatus {
  hasPassword: boolean;
  unlocked: boolean;
}

/** A `download-progress` event from the Rust side, for one add or store. */
export interface DownloadProgress {
  id: string;
  /** "download" while bytes arrive, then "save" while the copy goes to S3. */
  phase: "download" | "save";
  received: number;
  /** File size, when the site sends one. */
  total: number | null;
}

/** A newer version published on GitHub, from `check_update`. */
export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
}

/** An `update-progress` event while the new installer downloads. */
export interface UpdateProgress {
  received: number;
  total: number | null;
}

export type KindFilter = "all" | Exclude<MediaKind, "unknown">;
