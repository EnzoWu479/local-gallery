export type MediaKind = "image" | "audio" | "video" | "unknown";

export interface GalleryItem {
  id: number;
  url: string;
  kind: MediaKind;
  title: string | null;
  folderId: number | null;
  createdAt: string;
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

export type KindFilter = "all" | Exclude<MediaKind, "unknown">;
