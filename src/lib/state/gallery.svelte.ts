import * as api from "$lib/api";
import { displayTitle } from "$lib/media";
import type { DownloadProgress, Folder, GalleryItem, KindFilter } from "$lib/types";

function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}

/** "all" is the main wall: every item outside hidden folders. */
export type FolderSelection = number | "all";

class GalleryState {
  items = $state<GalleryItem[]>([]);
  folders = $state<Folder[]>([]);
  folder = $state<FolderSelection>("all");
  // Hidden folders need the password. The Rust side starts locked on every
  // launch and leaves hidden content out of its responses while locked.
  hasPassword = $state(false);
  unlocked = $state(false);
  filter = $state<KindFilter>("all");
  query = $state("");
  loading = $state(true);
  adding = $state(false);
  error = $state<string | null>(null);

  // Download progress, keyed by the id each add or store passes to Rust.
  #progress = $state<Record<string, DownloadProgress>>({});
  #listening = false;
  #addingId = $state<string | null>(null);
  #storingIds = $state<Record<number, string>>({});

  /** Progress of the link being added, once the first event arrives. */
  addProgress = $derived(this.#addingId ? (this.#progress[this.#addingId] ?? null) : null);

  /** Whether an item's media is being downloaded, and how far along. */
  storeProgress(itemId: number): DownloadProgress | null {
    const id = this.#storingIds[itemId];
    return id ? (this.#progress[id] ?? null) : null;
  }

  isStoring(itemId: number) {
    return itemId in this.#storingIds;
  }

  async #listen() {
    if (this.#listening) return;
    this.#listening = true;
    await api.onDownloadProgress((progress) => {
      this.#progress[progress.id] = progress;
    });
  }

  /** Runs a download with a fresh progress id, and forgets its progress after. */
  async #withProgress<T>(track: (id: string | null) => void, action: (id: string) => Promise<T>) {
    const id = crypto.randomUUID();
    track(id);
    try {
      return await this.#run(() => action(id));
    } finally {
      track(null);
      delete this.#progress[id];
    }
  }

  currentFolder = $derived(
    this.folder === "all" ? null : (this.folders.find((f) => f.id === this.folder) ?? null),
  );

  /** Items in the selected folder, before the kind filter and search. */
  scoped = $derived(
    this.folder === "all" ? this.items : this.items.filter((i) => i.folderId === this.folder),
  );

  visible = $derived.by(() => {
    const query = this.query.trim().toLowerCase();
    return this.scoped.filter((item) => {
      if (this.filter !== "all" && item.kind !== this.filter) return false;
      if (!query) return true;
      return (
        displayTitle(item).toLowerCase().includes(query) || item.url.toLowerCase().includes(query)
      );
    });
  });

  counts = $derived({
    all: this.scoped.length,
    image: this.scoped.filter((i) => i.kind === "image").length,
    audio: this.scoped.filter((i) => i.kind === "audio").length,
    video: this.scoped.filter((i) => i.kind === "video").length,
  });

  countIn(folderId: number) {
    return this.items.filter((i) => i.folderId === folderId).length;
  }

  async #run<T>(action: () => Promise<T>): Promise<T | undefined> {
    this.error = null;
    try {
      return await action();
    } catch (e) {
      this.error = errorMessage(e);
      return undefined;
    }
  }

  async load() {
    this.loading = true;
    this.#listen();
    await this.#run(async () => {
      const [items, folders, vault] = await Promise.all([
        api.listItems(),
        api.listFolders(),
        api.vaultStatus(),
      ]);
      this.items = items;
      this.folders = folders;
      this.hasPassword = vault.hasPassword;
      this.unlocked = vault.unlocked;
      if (this.folder !== "all" && !folders.some((f) => f.id === this.folder)) this.selectFolder("all");
    });
    this.loading = false;
  }

  selectFolder(folder: FolderSelection) {
    this.folder = folder;
    this.filter = "all";
  }

  // Hidden folders. These return an error message for the password dialog
  // instead of raising the global error toast.

  async #attempt(action: () => Promise<unknown>): Promise<string | null> {
    try {
      await action();
      await this.load();
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  }

  unlock(password: string) {
    return this.#attempt(() => api.unlock(password));
  }

  /** Creates the password (no `current`) or changes it. Unlocks either way. */
  setPassword(current: string | null, newPassword: string) {
    return this.#attempt(() => api.setPassword(current, newPassword));
  }

  async lock() {
    await this.#run(() => api.lock());
    await this.load();
  }

  // Items

  /** Adds the link to the selected folder. Returns true when it was added. */
  async add(url: string): Promise<boolean> {
    this.adding = true;
    const folderId = this.folder === "all" ? null : this.folder;
    const item = await this.#withProgress(
      (id) => (this.#addingId = id),
      (id) => api.addItem(url, folderId, id),
    );
    if (item) this.items = [item, ...this.items];
    this.adding = false;
    return !!item;
  }

  #replace(updated: GalleryItem) {
    this.items = this.items.map((i) => (i.id === updated.id ? updated : i));
  }

  /** Downloads and stores the media of an item saved before the S3 store. */
  async store(id: number) {
    const updated = await this.#withProgress(
      (progressId) => {
        if (progressId) this.#storingIds[id] = progressId;
        else delete this.#storingIds[id];
      },
      (progressId) => api.storeItem(id, progressId),
    );
    if (updated) this.#replace(updated);
  }

  async rename(id: number, title: string) {
    const updated = await this.#run(() => api.renameItem(id, title.trim() || null));
    if (updated) this.#replace(updated);
  }

  async move(id: number, folderId: number | null) {
    const updated = await this.#run(() => api.moveItem(id, folderId));
    if (updated) this.#replace(updated);
  }

  async remove(id: number) {
    const previous = this.items;
    this.items = this.items.filter((i) => i.id !== id);
    const done = await this.#run(() => api.deleteItem(id).then(() => true));
    if (!done) this.items = previous;
  }

  // Folders

  #sortFolders(folders: Folder[]) {
    return folders.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
  }

  async createFolder(name: string, hidden: boolean): Promise<boolean> {
    const folder = await this.#run(() => api.createFolder(name, hidden));
    if (!folder) return false;
    this.folders = this.#sortFolders([...this.folders, folder]);
    this.selectFolder(folder.id);
    return true;
  }

  async updateFolder(id: number, changes: { name?: string; hidden?: boolean }): Promise<boolean> {
    const updated = await this.#run(() => api.updateFolder(id, changes));
    if (!updated) return false;
    this.folders = this.#sortFolders(this.folders.map((f) => (f.id === id ? updated : f)));
    return true;
  }

  async deleteFolder(id: number) {
    const done = await this.#run(() => api.deleteFolder(id).then(() => true));
    if (!done) return;
    this.folders = this.folders.filter((f) => f.id !== id);
    this.items = this.items.map((i) => (i.folderId === id ? { ...i, folderId: null } : i));
    if (this.folder === id) this.selectFolder("all");
  }
}

export const gallery = new GalleryState();
