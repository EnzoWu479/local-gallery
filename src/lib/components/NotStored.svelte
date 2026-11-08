<script lang="ts">
  import { progressFraction, progressLabel } from "$lib/media";
  import { gallery } from "$lib/state/gallery.svelte";
  import type { GalleryItem } from "$lib/types";
  import Icon from "./Icon.svelte";
  import ProgressBar from "./ProgressBar.svelte";

  // An item saved before media was copied to the S3 store. Media is only shown
  // from the store, so it needs downloading first.
  let { item, tone = "mat" }: { item: GalleryItem; tone?: "mat" | "wall" } = $props();

  let busy = $derived(gallery.isStoring(item.id));
  let progress = $derived(gallery.storeProgress(item.id));
</script>

<div class={["flex w-full max-w-48 flex-col items-center gap-3 text-center", tone === "mat" ? "text-wall/60" : "text-ink-muted"]}>
  <Icon name="image" size={28} />
  <p class="text-xs">Esta mídia ainda não foi salva na galeria.</p>
  <button
    onclick={() => gallery.store(item.id)}
    disabled={busy}
    class={[
      "flex items-center gap-1.5 rounded-sm px-3 py-1 text-xs font-medium tabular-nums disabled:opacity-80",
      tone === "mat" ? "bg-wall-deep text-mat" : "bg-mat text-wall-deep",
    ]}
  >
    {#if busy}<Icon name="loader" size={13} class="animate-spin" />{/if}
    {busy ? progressLabel(progress) : "Baixar agora"}
  </button>
  {#if busy}
    <ProgressBar fraction={progressFraction(progress)} label="Progresso do download" />
  {/if}
</div>
