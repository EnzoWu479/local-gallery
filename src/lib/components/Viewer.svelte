<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { mediaSrc } from "$lib/api";
  import { displayTitle, isLocal, sourceOf } from "$lib/media";
  import { player } from "$lib/state/player";
  import type { GalleryItem } from "$lib/types";
  import Icon from "./Icon.svelte";
  import NotStored from "./NotStored.svelte";
  import { kindLabel } from "./Piece.svelte";
  import RecordArt from "./RecordArt.svelte";

  let { items, index = $bindable() }: { items: GalleryItem[]; index: number | null } = $props();

  let item = $derived(index === null ? null : items[index]);

  function step(delta: number) {
    if (index === null || items.length === 0) return;
    index = (index + delta + items.length) % items.length;
  }

  function onkeydown(event: KeyboardEvent) {
    if (index === null) return;
    if (event.key === "Escape") index = null;
    if (event.key === "ArrowLeft") step(-1);
    if (event.key === "ArrowRight") step(1);
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }
</script>

<svelte:window {onkeydown} />

{#if item}
  <div
    class="viewer fixed inset-0 z-40 flex flex-col bg-wall-deep"
    role="dialog"
    aria-modal="true"
    aria-label={displayTitle(item)}
    tabindex="-1"
    use:focusOnMount
  >
    <div class="relative flex min-h-0 flex-1 items-center justify-center px-16 pt-10 pb-4">
      {#key item.id}
        {#if item.kind !== "unknown" && !item.stored}
          <NotStored {item} tone="wall" />
        {:else if item.kind === "image"}
          <img src={mediaSrc(item)} alt={displayTitle(item)} class="max-h-full max-w-full object-contain shadow-2xl" />
        {:else if item.kind === "video"}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            src={mediaSrc(item)}
            controls
            autoplay
            onplay={(e) => player.claim(e.currentTarget)}
            class="max-h-full max-w-full shadow-2xl"
          ></video>
        {:else if item.kind === "audio"}
          <div class="aspect-square h-full max-h-[min(70vh,520px)] bg-mat p-[8%] shadow-2xl">
            <RecordArt {item} large />
          </div>
        {:else}
          <div class="flex flex-col items-center gap-4 text-center">
            <Icon name="link" size={36} class="text-ink-muted" />
            <p class="max-w-md text-ink-muted">
              Não reconhecemos esse link como imagem, áudio ou vídeo. Abra no navegador para ver o conteúdo.
            </p>
          </div>
        {/if}
      {/key}

      {#if items.length > 1}
        <button
          onclick={() => step(-1)}
          aria-label="Anterior"
          class="absolute top-1/2 left-3 -translate-y-1/2 rounded-full p-2 text-ink-muted hover:bg-wall hover:text-ink"
        >
          <Icon name="left" size={28} />
        </button>
        <button
          onclick={() => step(1)}
          aria-label="Próxima"
          class="absolute top-1/2 right-3 -translate-y-1/2 rounded-full p-2 text-ink-muted hover:bg-wall hover:text-ink"
        >
          <Icon name="right" size={28} />
        </button>
      {/if}
    </div>

    <footer class="flex items-end justify-between gap-6 border-t border-wall-line px-6 py-4">
      <div class="min-w-0">
        <h2 class="truncate font-serif text-xl text-ink italic">{displayTitle(item)}</h2>
        <p class="truncate text-xs text-ink-muted">
          {kindLabel[item.kind]}, {sourceOf(item)}
          <span class="ml-2 tabular-nums">{(index ?? 0) + 1} de {items.length}</span>
        </p>
      </div>
      <div class="flex shrink-0 gap-2">
        {#if !isLocal(item)}
          <button
            onclick={() => openUrl(item.url)}
            class="flex items-center gap-2 rounded-sm border border-wall-line px-3 py-1.5 text-ink hover:border-ink-muted"
          >
            <Icon name="external" size={15} />
            Abrir no navegador
          </button>
        {/if}
        <button
          onclick={() => (index = null)}
          class="flex items-center gap-2 rounded-sm bg-mat px-3 py-1.5 font-medium text-wall-deep hover:bg-white"
        >
          <Icon name="x" size={15} />
          Fechar
        </button>
      </div>
    </footer>
  </div>
{/if}

<style>
  .viewer {
    animation: open 160ms ease-out;
  }

  @keyframes open {
    from {
      opacity: 0;
    }
  }
</style>
