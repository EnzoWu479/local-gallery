import type { DownloadProgress, GalleryItem } from "./types";

export function isHttpUrl(text: string): boolean {
  try {
    const url = new URL(text.trim());
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

/** The custom title, or the file name from the URL, or the host as a last resort. */
export function displayTitle(item: GalleryItem): string {
  if (item.title) return item.title;
  try {
    const url = new URL(item.url);
    const segment = url.pathname.split("/").filter(Boolean).pop();
    return segment ? decodeURIComponent(segment) : url.host;
  } catch {
    return item.url;
  }
}

/** Whether the item came from a file on this computer instead of a link. */
export function isLocal(item: GalleryItem): boolean {
  return item.url.startsWith("local:") || item.url.startsWith("blob:");
}

/** Where the item came from, for its wall label: the link's host or "Arquivo local". */
export function sourceOf(item: GalleryItem): string {
  return isLocal(item) ? "Arquivo local" : hostOf(item.url);
}

export function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/** Pull the first http(s) link out of pasted or dropped text. */
export function extractUrl(text: string): string | null {
  const match = text.match(/https?:\/\/\S+/);
  return match && isHttpUrl(match[0]) ? match[0] : null;
}

/** A stable muted color per item, used for the record label of audio pieces. */
export function coverColor(id: number): string {
  return `hsl(${(id * 47) % 360} 38% 46%)`;
}

function formatMegabytes(bytes: number): string {
  return `${(bytes / 1024 / 1024).toLocaleString("pt-BR", { maximumFractionDigits: 1 })} MB`;
}

/** Button text for a download in progress: "Baixando 42%", "Baixando 3,2 MB" or "Salvando". */
export function progressLabel(progress: DownloadProgress | null, waiting = "Baixando"): string {
  if (!progress) return waiting;
  if (progress.phase === "save") return "Salvando";
  if (progress.total) return `Baixando ${Math.floor((progress.received / progress.total) * 100)}%`;
  return `Baixando ${formatMegabytes(progress.received)}`;
}

/** Share done, from 0 to 1, or null when the size is unknown. */
export function progressFraction(progress: DownloadProgress | null): number | null {
  if (!progress) return null;
  if (progress.phase === "save") return 1;
  return progress.total ? Math.min(progress.received / progress.total, 1) : null;
}

export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds)) return "0:00";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}
