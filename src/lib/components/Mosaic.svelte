<script lang="ts" module>
  export const mosaicLayouts = [
    { id: "1x2", cols: 2, rows: 1, label: "Lado a lado" },
    { id: "2x2", cols: 2, rows: 2, label: "2×2" },
    { id: "3x3", cols: 3, rows: 3, label: "3×3" },
    { id: "4x4", cols: 4, rows: 4, label: "4×4" },
  ] as const;
  export type MosaicLayout = (typeof mosaicLayouts)[number]["id"];

  const STORAGE_KEY = "mosaic";
  const MAX_SLOTS = 16;

  interface Saved {
    layout: MosaicLayout;
    slots: (number | null)[];
    fill: boolean;
  }

  function loadSaved(): Saved {
    const empty: Saved = { layout: "2x2", slots: Array(MAX_SLOTS).fill(null), fill: false };
    try {
      const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
      if (!raw) return empty;
      // Older saves kept only the square size as a number.
      const id = typeof raw.size === "number" ? `${raw.size}x${raw.size}` : raw.layout;
      const layout = mosaicLayouts.find((l) => l.id === id)?.id ?? "2x2";
      const slots = Array.from({ length: MAX_SLOTS }, (_, i) =>
        typeof raw.slots?.[i] === "number" ? raw.slots[i] : null,
      );
      return { layout, slots, fill: !!raw.fill };
    } catch {
      return empty;
    }
  }
</script>

<script lang="ts">
  import { mediaSrc } from "$lib/api";
  import { displayTitle } from "$lib/media";
  import { gallery } from "$lib/state/gallery.svelte";
  import { player } from "$lib/state/player";
  import type { GalleryItem } from "$lib/types";
  import Icon from "./Icon.svelte";
  import MosaicVideo from "./MosaicVideo.svelte";

  // A wall of two cells side by side, or 2x2, 3x3 or 4x4, each holding one image or video. The
  // videos share one transport: they all play, pause and restart together.
  let { open = $bindable() }: { open: boolean } = $props();

  const saved = loadSaved();
  let layoutId = $state<MosaicLayout>(saved.layout);
  let layout = $derived(mosaicLayouts.find((l) => l.id === layoutId) ?? mosaicLayouts[1]);
  let slots = $state<(number | null)[]>(saved.slots);
  let fill = $state(saved.fill);

  let playing = $state(false);
  let allMuted = $state(true);
  let picking = $state<number | null>(null);
  let query = $state("");

  const videos = new Set<HTMLVideoElement>();

  $effect(() => {
    const data: Saved = { layout: layoutId, slots: $state.snapshot(slots), fill };
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
    } catch {
      // Storage is a convenience; the mosaic works without it.
    }
  });

  // Taking over the screen silences anything else that was playing.
  $effect(() => {
    if (open) player.release();
    else {
      playing = false;
      picking = null;
    }
  });

  /** Only stored images and videos can hang in a cell. */
  let candidates = $derived(
    gallery.items.filter((i) => i.stored && (i.kind === "image" || i.kind === "video")),
  );

  let pickerItems = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return candidates;
    return candidates.filter(
      (i) => displayTitle(i).toLowerCase().includes(q) || i.url.toLowerCase().includes(q),
    );
  });

  /** The item in each visible cell, or null when empty or no longer available. */
  let cells = $derived(
    slots
      .slice(0, layout.cols * layout.rows)
      .map((id) => (id === null ? null : (candidates.find((i) => i.id === id) ?? null))),
  );

  let hasVideo = $derived(cells.some((c) => c?.kind === "video"));

  /** Adds a video to the shared transport; returns its cleanup. */
  function register(node: HTMLVideoElement) {
    videos.add(node);
    // A video that joins while the others play starts with them.
    const join = () => {
      if (playing) node.play().catch(() => {});
    };
    node.addEventListener("loadeddata", join);
    return () => {
      node.removeEventListener("loadeddata", join);
      videos.delete(node);
    };
  }

  function play() {
    playing = true;
    for (const v of videos) v.play().catch(() => {});
  }

  function pause() {
    playing = false;
    for (const v of videos) v.pause();
  }

  function toggle() {
    if (playing) pause();
    else play();
  }

  function restart() {
    for (const v of videos) v.currentTime = 0;
  }

  function pick(item: GalleryItem) {
    if (picking === null) return;
    slots[picking] = item.id;
    picking = null;
    query = "";
  }

  function clearSlot(index: number) {
    slots[index] = null;
  }

  function onkeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === "Escape") {
      if (picking !== null) picking = null;
      else open = false;
      return;
    }
    if (picking !== null) return;
    // A focused button handles space by itself; don't toggle twice.
    if (event.target instanceof HTMLButtonElement || event.target instanceof HTMLInputElement) return;
    if (event.key === " " && hasVideo) {
      event.preventDefault();
      toggle();
    }
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }
</script>

<svelte:window {onkeydown} />

{#if open}
  <div
    class="mosaic fixed inset-0 z-40 flex flex-col bg-wall-deep"
    role="dialog"
    aria-modal="true"
    aria-label="Mosaico"
    tabindex="-1"
    use:focusOnMount
  >
    <header class="flex shrink-0 flex-wrap items-center gap-x-6 gap-y-3 border-b border-wall-line px-6 py-3">
      <h2 class="font-serif text-xl text-ink">Mosaico</h2>

      <div class="flex rounded-sm border border-wall-line" role="radiogroup" aria-label="Tamanho da grade">
        {#each mosaicLayouts as option (option.id)}
          <button
            role="radio"
            aria-checked={layoutId === option.id}
            onclick={() => (layoutId = option.id)}
            class={[
              "px-3 py-1 tabular-nums transition-colors",
              layoutId === option.id ? "bg-mat font-medium text-wall-deep" : "text-ink-muted hover:text-ink",
            ]}
          >
            {option.label}
          </button>
        {/each}
      </div>

      <button
        onclick={() => (fill = !fill)}
        aria-pressed={fill}
        class="rounded-sm border border-wall-line px-3 py-1 text-ink-muted hover:text-ink aria-pressed:border-brass aria-pressed:text-ink"
      >
        Preencher células
      </button>

      <div class="flex items-center gap-1" aria-label="Controles dos vídeos">
        <button
          onclick={toggle}
          disabled={!hasVideo}
          class="flex items-center gap-2 rounded-sm bg-mat px-3 py-1 font-medium text-wall-deep hover:bg-white disabled:opacity-35"
        >
          <Icon name={playing ? "pause" : "play"} size={14} />
          {playing ? "Pausar todos" : "Tocar todos"}
        </button>
        <button
          onclick={restart}
          disabled={!hasVideo}
          aria-label="Voltar todos ao início"
          title="Voltar todos ao início"
          class="rounded-sm p-1.5 text-ink-muted hover:bg-wall hover:text-ink disabled:opacity-35"
        >
          <Icon name="restart" size={16} />
        </button>
        <button
          onclick={() => (allMuted = !allMuted)}
          disabled={!hasVideo}
          aria-label={allMuted ? "Ativar som de todos" : "Silenciar todos"}
          title={allMuted ? "Ativar som de todos" : "Silenciar todos"}
          class="rounded-sm p-1.5 text-ink-muted hover:bg-wall hover:text-ink disabled:opacity-35"
        >
          <Icon name={allMuted ? "volumeOff" : "volume"} size={16} />
        </button>
      </div>

      <button
        onclick={() => (open = false)}
        class="ml-auto flex items-center gap-2 rounded-sm border border-wall-line px-3 py-1 text-ink hover:border-ink-muted"
      >
        <Icon name="x" size={15} />
        Fechar
      </button>
    </header>

    <div
      class="grid min-h-0 flex-1 gap-1 p-1"
      style:grid-template-columns="repeat({layout.cols}, minmax(0, 1fr))"
      style:grid-template-rows="repeat({layout.rows}, minmax(0, 1fr))"
    >
      {#each cells as item, i (i)}
        <div class="cell group @container relative min-h-0 min-w-0 overflow-hidden bg-black">
          {#if item}
            {#key item.id}
              {#if item.kind === "video"}
                <MosaicVideo {item} {fill} {playing} {allMuted} {register} ontoggle={toggle} />
              {:else}
                <img
                  src={mediaSrc(item)}
                  alt={displayTitle(item)}
                  draggable="false"
                  class={["absolute inset-0 size-full", fill ? "object-cover" : "object-contain"]}
                />
              {/if}
            {/key}

            <div
              class="absolute top-2 right-2 flex items-center rounded-sm bg-mat text-wall/70 opacity-0 shadow-sm transition-opacity group-focus-within:opacity-100 group-hover:opacity-100"
            >
              <button onclick={() => (picking = i)} class="p-1 hover:text-wall-deep" aria-label="Trocar mídia" title="Trocar mídia">
                <Icon name="pencil" size={15} />
              </button>
              <button onclick={() => clearSlot(i)} class="p-1 hover:text-wall-deep" aria-label="Esvaziar célula" title="Esvaziar célula">
                <Icon name="x" size={15} />
              </button>
            </div>
            <p class="pointer-events-none absolute top-2 left-2 max-w-[calc(100%-5rem)] truncate rounded-sm bg-wall-deep/70 px-2 py-0.5 font-serif text-sm text-ink italic opacity-0 transition-opacity group-hover:opacity-100">
              {displayTitle(item)}
            </p>
          {:else}
            <button
              onclick={() => (picking = i)}
              class="flex size-full flex-col items-center justify-center gap-2 border border-dashed border-wall-line bg-wall-deep text-ink-muted hover:border-brass hover:text-ink"
            >
              <Icon name="plus" size={24} />
              <span class="text-xs">Escolher mídia</span>
            </button>
          {/if}
        </div>
      {/each}
    </div>

    {#if picking !== null}
      <div
        class="absolute inset-0 z-10 flex items-center justify-center bg-black/60 p-6"
        role="presentation"
        onclick={(e) => {
          if (e.target === e.currentTarget) picking = null;
        }}
      >
        <div
          class="flex max-h-full w-full max-w-3xl flex-col rounded-sm border border-wall-line bg-wall shadow-2xl"
          role="dialog"
          aria-label="Escolher mídia para a célula {picking + 1}"
        >
          <div class="flex items-center gap-4 border-b border-wall-line px-5 py-3">
            <h3 class="font-serif text-lg text-ink">Escolher mídia</h3>
            <label class="relative flex-1">
              <span class="sr-only">Buscar</span>
              <Icon name="search" size={14} class="pointer-events-none absolute top-1/2 left-0 -translate-y-1/2 text-ink-muted" />
              <!-- svelte-ignore a11y_autofocus -->
              <input
                bind:value={query}
                type="search"
                autofocus
                placeholder="Buscar por nome ou link"
                class="h-8 w-full border-b border-wall-line bg-transparent pl-6 text-ink placeholder:text-ink-muted/70 focus:border-brass focus:outline-none"
              />
            </label>
            <button onclick={() => (picking = null)} aria-label="Fechar" class="text-ink-muted hover:text-ink">
              <Icon name="x" size={18} />
            </button>
          </div>

          <div class="min-h-0 overflow-y-auto p-5">
            {#if pickerItems.length === 0}
              <p class="text-ink-muted">
                {candidates.length ? "Nada encontrado com essa busca." : "Nenhuma imagem ou vídeo guardado ainda."}
              </p>
            {:else}
              <ul class="grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] gap-4">
                {#each pickerItems as option (option.id)}
                  <li>
                    <button onclick={() => pick(option)} class="group/opt block w-full text-left">
                      <span
                        class={[
                          "relative flex aspect-square items-center justify-center overflow-hidden bg-mat p-[8%]",
                          slots[picking] === option.id && "ring-2 ring-brass",
                        ]}
                      >
                        {#if option.kind === "video"}
                          <!-- svelte-ignore a11y_media_has_caption -->
                          <video src={mediaSrc(option)} preload="metadata" muted class="max-h-full max-w-full object-contain"></video>
                          <span class="absolute flex size-8 items-center justify-center rounded-full bg-wall-deep/80 text-mat">
                            <Icon name="play" size={13} class="translate-x-px" />
                          </span>
                        {:else}
                          <img src={mediaSrc(option)} alt="" loading="lazy" class="max-h-full max-w-full object-contain" />
                        {/if}
                      </span>
                      <span class="mt-1.5 block truncate font-serif text-sm text-ink italic group-hover/opt:underline">
                        {displayTitle(option)}
                      </span>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .mosaic {
    animation: open 160ms ease-out;
  }

  @keyframes open {
    from {
      opacity: 0;
    }
  }
</style>
