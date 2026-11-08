import type { DownloadProgress, Folder, GalleryItem, MediaKind } from "./types";

// Dev-only stand-in for the Rust commands. Kind detection is by extension only.

const now = () => new Date().toISOString();

let folders: Folder[] = [
  { id: 1, name: "Paisagens", hidden: false, createdAt: now() },
  { id: 2, name: "Rascunhos", hidden: true, createdAt: now() },
];
let nextFolderId = 3;

const seed: [string, MediaKind, number | null][] = [
  ["https://picsum.photos/id/1015/900/600.jpg", "image", 1],
  ["https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3", "audio", null],
  ["https://picsum.photos/id/1025/600/900.jpg", "image", 2],
  ["https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4", "video", null],
  ["https://picsum.photos/id/1043/800/800.jpg", "image", 1],
  ["https://www.soundhelix.com/examples/mp3/SoundHelix-Song-8.mp3", "audio", 2],
  ["https://picsum.photos/id/1069/1200/500.jpg", "image", null],
  ["https://example.com/some-page", "unknown", null],
];

// No password until one is created, like a fresh database.
let password: string | null = null;
let unlocked = false;

const hiddenFolderIds = () => new Set(folders.filter((f) => f.hidden).map((f) => f.id));
const isVisible = (folderId: number | null) => unlocked || folderId === null || !hiddenFolderIds().has(folderId);

let nextId = 1;
let items: GalleryItem[] = seed
  .map(([url, kind, folderId]) => ({
    id: nextId++,
    url,
    kind,
    title: null,
    folderId,
    createdAt: now(),
    stored: kind !== "unknown",
  }))
  .reverse();

// Saved before the S3 store existed: shows the "Baixar agora" state.
items.find((i) => i.url.includes("/1069/"))!.stored = false;

function kindOf(url: string): MediaKind {
  const ext = new URL(url).pathname.split(".").pop()?.toLowerCase() ?? "";
  if (["jpg", "jpeg", "png", "gif", "webp", "avif", "svg"].includes(ext)) return "image";
  if (["mp3", "wav", "ogg", "flac", "m4a", "aac", "opus"].includes(ext)) return "audio";
  if (["mp4", "webm", "mov"].includes(ext)) return "video";
  return "unknown";
}

function patchItem(id: number, changes: Partial<GalleryItem>) {
  items = items.map((i) => (i.id === id ? { ...i, ...changes } : i));
  return items.find((i) => i.id === id);
}

function checkFolderName(name: string, exceptId?: number) {
  if (!name.trim()) throw "Dê um nome para a pasta.";
  if (folders.some((f) => f.name === name.trim() && f.id !== exceptId)) throw "Já existe uma pasta com esse nome.";
}

const progressHandlers = new Set<(progress: DownloadProgress) => void>();

export function onMockProgress(handler: (progress: DownloadProgress) => void) {
  progressHandlers.add(handler);
  return () => progressHandlers.delete(handler);
}

/** Pretends to download 8 MB in about two seconds, then save it. */
async function simulateDownload(id: string | undefined) {
  if (!id) return;
  const emit = (progress: DownloadProgress) => progressHandlers.forEach((h) => h(progress));
  const total = 8 * 1024 * 1024;
  for (let received = 0; received <= total; received += total / 25) {
    emit({ id, phase: "download", received, total });
    await new Promise((r) => setTimeout(r, 80));
  }
  emit({ id, phase: "save", received: total, total });
  await new Promise((r) => setTimeout(r, 400));
}

export async function mockInvoke<T>(cmd: string, args: any = {}): Promise<T> {
  await new Promise((r) => setTimeout(r, 150));
  switch (cmd) {
    case "list_items":
      return items.filter((i) => isVisible(i.folderId)) as T;
    case "add_item": {
      if (items.some((i) => i.url === args.url)) throw "Esse link já está na galeria.";
      const item = {
        id: nextId++,
        url: args.url,
        kind: kindOf(args.url),
        title: null,
        folderId: args.folderId ?? null,
        createdAt: now(),
        stored: true,
      };
      if (item.kind === "unknown") throw "Esse link não é uma imagem, áudio ou vídeo.";
      await simulateDownload(args.progressId);
      items = [item, ...items];
      return item as T;
    }
    case "store_item":
      await simulateDownload(args.progressId);
      return patchItem(args.id, { stored: true }) as T;
    case "rename_item":
      return patchItem(args.id, { title: args.title }) as T;
    case "move_item":
      return patchItem(args.id, { folderId: args.folderId }) as T;
    case "delete_item":
      items = items.filter((i) => i.id !== args.id);
      return undefined as T;
    case "list_folders":
      return folders.filter((f) => unlocked || !f.hidden).sort((a, b) => a.name.localeCompare(b.name)) as T;
    case "create_folder": {
      checkFolderName(args.name);
      if (args.hidden && !unlocked) throw "Desbloqueie as pastas ocultas para continuar.";
      const folder = { id: nextFolderId++, name: args.name.trim(), hidden: args.hidden, createdAt: now() };
      folders = [...folders, folder];
      return folder as T;
    }
    case "update_folder": {
      if (args.name !== undefined) checkFolderName(args.name, args.id);
      folders = folders.map((f) =>
        f.id === args.id ? { ...f, name: args.name?.trim() ?? f.name, hidden: args.hidden ?? f.hidden } : f,
      );
      return folders.find((f) => f.id === args.id) as T;
    }
    case "delete_folder":
      folders = folders.filter((f) => f.id !== args.id);
      items = items.map((i) => (i.folderId === args.id ? { ...i, folderId: null } : i));
      return undefined as T;
    case "vault_status":
      return { hasPassword: password !== null, unlocked } as T;
    case "set_password":
      if (args.newPassword.length < 4) throw "A senha precisa ter pelo menos 4 caracteres.";
      if (password !== null && args.current !== password) throw "A senha atual está incorreta.";
      password = args.newPassword;
      unlocked = true;
      return undefined as T;
    case "unlock":
      if (password === null) throw "Crie uma senha primeiro.";
      if (args.password !== password) {
        await new Promise((r) => setTimeout(r, 800));
        throw "Senha incorreta.";
      }
      unlocked = true;
      return undefined as T;
    case "lock":
      unlocked = false;
      return undefined as T;
    default:
      throw `Comando desconhecido: ${cmd}`;
  }
}
