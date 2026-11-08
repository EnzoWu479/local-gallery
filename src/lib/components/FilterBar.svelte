<script lang="ts">
  import { gallery } from "$lib/state/gallery.svelte";
  import type { KindFilter } from "$lib/types";
  import Icon from "./Icon.svelte";

  const filters: { value: KindFilter; label: string }[] = [
    { value: "all", label: "Todas" },
    { value: "image", label: "Imagens" },
    { value: "audio", label: "Áudios" },
    { value: "video", label: "Vídeos" },
  ];
</script>

<div class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
  <div class="flex gap-5" role="tablist" aria-label="Filtrar por tipo">
    {#each filters as filter (filter.value)}
      {@const active = gallery.filter === filter.value}
      <button
        role="tab"
        aria-selected={active}
        onclick={() => (gallery.filter = filter.value)}
        class={[
          "-mb-px border-b-2 pb-2 transition-colors",
          active ? "border-brass text-ink" : "border-transparent text-ink-muted hover:text-ink",
        ]}
      >
        {filter.label}
        <span class="ml-0.5 text-xs text-ink-muted tabular-nums">{gallery.counts[filter.value]}</span>
      </button>
    {/each}
  </div>

  <label class="relative mb-2 w-full sm:w-60">
    <span class="sr-only">Buscar</span>
    <Icon
      name="search"
      size={14}
      class="pointer-events-none absolute top-1/2 left-0 -translate-y-1/2 text-ink-muted"
    />
    <input
      bind:value={gallery.query}
      type="search"
      placeholder="Buscar por nome ou link"
      class="h-8 w-full border-b border-wall-line bg-transparent pl-6 text-ink placeholder:text-ink-muted/70 focus:border-brass focus:outline-none"
    />
  </label>
</div>
