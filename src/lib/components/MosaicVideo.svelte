<script lang="ts">
  import { mediaSrc } from "$lib/api";
  import { displayTitle, formatTime } from "$lib/media";
  import type { GalleryItem } from "$lib/types";
  import Icon from "./Icon.svelte";

  // One video of the mosaic. Play and pause belong to the whole mosaic, but
  // each video keeps its own position and volume.
  let {
    item,
    fill,
    playing,
    allMuted,
    register,
    ontoggle,
  }: {
    item: GalleryItem;
    fill: boolean;
    playing: boolean;
    /** The mosaic's mute button; changing it overrides each video's own. */
    allMuted: boolean;
    register: (video: HTMLVideoElement) => () => void;
    ontoggle: () => void;
  } = $props();

  let video = $state<HTMLVideoElement>();
  let currentTime = $state(0);
  let duration = $state(0);
  let volume = $state(1);
  let muted = $state(true);

  $effect(() => {
    muted = allMuted;
  });

  $effect(() => {
    if (video) return register(video);
  });

  function seek(event: Event) {
    if (video) video.currentTime = Number((event.currentTarget as HTMLInputElement).value);
  }

  function setVolume(event: Event) {
    volume = Number((event.currentTarget as HTMLInputElement).value);
    muted = volume === 0;
  }
</script>

<!-- svelte-ignore a11y_media_has_caption -->
<video
  bind:this={video}
  bind:currentTime
  bind:duration
  bind:volume
  bind:muted
  src={mediaSrc(item)}
  loop
  playsinline
  preload="auto"
  class={["absolute inset-0 size-full", fill ? "object-cover" : "object-contain"]}
></video>

<button
  onclick={ontoggle}
  class="absolute inset-0 flex items-center justify-center"
  aria-label={playing ? "Pausar todos os vídeos" : "Tocar todos os vídeos"}
>
  {#if !playing}
    <span class="flex size-12 items-center justify-center rounded-full bg-wall-deep/80 text-mat">
      <Icon name="play" size={20} class="translate-x-px" />
    </span>
  {/if}
</button>

<div
  class="absolute inset-x-0 bottom-0 flex items-center gap-2 bg-gradient-to-t from-black/80 to-transparent px-2 pt-6 pb-1.5 text-mat opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100"
>
  <span class="shrink-0 text-xs tabular-nums @max-[260px]:hidden">{formatTime(currentTime)}</span>
  <input
    type="range"
    min="0"
    max={duration || 0}
    step="0.1"
    value={currentTime}
    oninput={seek}
    aria-label="Posição de {displayTitle(item)}"
    class="min-w-0 flex-1 accent-brass"
  />
  <span class="shrink-0 text-xs tabular-nums @max-[260px]:hidden">{formatTime(duration)}</span>
  <button
    onclick={() => (muted = !muted)}
    aria-label={muted ? "Ativar som deste vídeo" : "Silenciar este vídeo"}
    title={muted ? "Ativar som" : "Silenciar"}
    class="shrink-0 rounded-sm p-0.5 hover:bg-white/15"
  >
    <Icon name={muted || volume === 0 ? "volumeOff" : "volume"} size={15} />
  </button>
  <input
    type="range"
    min="0"
    max="1"
    step="0.05"
    value={muted ? 0 : volume}
    oninput={setVolume}
    aria-label="Volume de {displayTitle(item)}"
    class="w-16 shrink-0 accent-brass @max-[200px]:hidden"
  />
</div>
