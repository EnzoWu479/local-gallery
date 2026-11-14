<script lang="ts">
  import { onMount } from "svelte";
  import AddLinkBar from "$lib/components/AddLinkBar.svelte";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import FilterBar from "$lib/components/FilterBar.svelte";
  import FolderNav from "$lib/components/FolderNav.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Mosaic from "$lib/components/Mosaic.svelte";
  import Piece from "$lib/components/Piece.svelte";
  import UpdateBanner from "$lib/components/UpdateBanner.svelte";
  import Viewer from "$lib/components/Viewer.svelte";
  import { extractUrl } from "$lib/media";
  import { gallery } from "$lib/state/gallery.svelte";

  let viewerIndex = $state<number | null>(null);
  let dragging = $state(false);
  let mosaicOpen = $state(false);

  onMount(() => {
    gallery.load();
  });

  // Errors dismiss themselves after a while.
  $effect(() => {
    if (!gallery.error) return;
    const timer = setTimeout(() => (gallery.error = null), 6000);
    return () => clearTimeout(timer);
  });

  function isTyping(target: EventTarget | null) {
    return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
  }

  function onpaste(event: ClipboardEvent) {
    if (isTyping(event.target)) return;
    const url = extractUrl(event.clipboardData?.getData("text") ?? "");
    if (url) {
      event.preventDefault();
      gallery.add(url);
    }
  }

  function ondragover(event: DragEvent) {
    event.preventDefault();
    dragging = true;
  }

  function ondrop(event: DragEvent) {
    event.preventDefault();
    dragging = false;
    const data = event.dataTransfer;
    const url = extractUrl(data?.getData("text/uri-list") || data?.getData("text/plain") || "");
    if (url) gallery.add(url);
    else gallery.error = "Arraste um link (http ou https) para adicionar.";
  }

  function ondragleave(event: DragEvent) {
    if (!event.relatedTarget) dragging = false;
  }
</script>

<svelte:window {onpaste} {ondragover} {ondrop} {ondragleave} />

<div class="flex h-screen flex-col">
  <header class="flex shrink-0 flex-wrap items-center gap-x-10 gap-y-4 border-b border-wall-line px-6 py-5">
    <h1 class="font-serif text-[26px] leading-none text-ink">Local Gallery</h1>
    <div class="min-w-64 flex-1">
      <AddLinkBar />
    </div>
  </header>

  <div class="flex min-h-0 flex-1 flex-col md:flex-row">
    <aside
      class="max-h-56 shrink-0 overflow-y-auto border-b border-wall-line bg-wall-deep md:max-h-none md:w-56 md:border-r md:border-b-0"
    >
      <FolderNav />
    </aside>

    <main class="min-w-0 flex-1 overflow-y-auto px-6 pb-10 sm:px-10">
      <div class="sticky top-0 z-20 -mx-6 bg-wall/95 px-6 pt-6 backdrop-blur sm:-mx-10 sm:px-10">
        <div class="flex items-baseline gap-3">
          <h2 class="truncate font-serif text-2xl text-ink">{gallery.currentFolder?.name ?? "Tudo"}</h2>
          {#if gallery.currentFolder?.hidden}
            <span class="flex shrink-0 items-center gap-1.5 text-xs text-ink-muted">
              <Icon name="eyeOff" size={13} />
              Pasta oculta
            </span>
          {/if}
          <button
            onclick={() => (mosaicOpen = true)}
            class="ml-auto flex shrink-0 items-center gap-2 self-center rounded-sm border border-wall-line px-3 py-1 text-ink-muted hover:border-ink-muted hover:text-ink"
          >
            <Icon name="grid" size={15} />
            Mosaico
          </button>
        </div>
        <div class="mt-4 border-b border-wall-line">
          <FilterBar />
        </div>
      </div>

      <div class="pt-8">
        {#if gallery.loading}
          <p class="flex items-center gap-2 text-ink-muted">
            <Icon name="loader" size={16} class="animate-spin" />
            Carregando a galeria
          </p>
        {:else if gallery.visible.length === 0}
          <EmptyState
            variant={gallery.scoped.length ? "no-match" : gallery.currentFolder ? "empty-folder" : "empty-wall"}
            query={gallery.query.trim()}
          />
        {:else}
          <div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-x-8 gap-y-10">
            {#each gallery.visible as item, i (item.id)}
              <Piece {item} onopen={() => (viewerIndex = i)} />
            {/each}
          </div>
        {/if}
      </div>
    </main>
  </div>
</div>

<Viewer items={gallery.visible} bind:index={viewerIndex} />

<Mosaic bind:open={mosaicOpen} />

<UpdateBanner />

{#if dragging}
  <div class="pointer-events-none fixed inset-4 z-50 flex items-center justify-center border-2 border-dashed border-brass bg-wall-deep/80">
    <p class="font-serif text-2xl text-ink">Solte o link para adicionar à parede</p>
  </div>
{/if}

{#if gallery.error}
  <div
    role="alert"
    class="fixed bottom-6 left-6 z-50 flex max-w-md items-start gap-3 rounded-sm bg-mat px-4 py-3 text-wall-deep shadow-xl"
  >
    <p class="flex-1">{gallery.error}</p>
    <button onclick={() => (gallery.error = null)} aria-label="Fechar aviso" class="opacity-60 hover:opacity-100">
      <Icon name="x" size={16} />
    </button>
  </div>
{/if}
