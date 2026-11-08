<script lang="ts">
  import { mediaSrc } from "$lib/api";
  import { coverColor, formatTime } from "$lib/media";
  import { player } from "$lib/state/player";
  import type { GalleryItem } from "$lib/types";
  import Icon from "./Icon.svelte";

  // An audio piece drawn as a record: the label holds the play button, the
  // disc turns while it plays.
  let { item, large = false }: { item: GalleryItem; large?: boolean } = $props();

  let audio = $state<HTMLAudioElement>();
  let paused = $state(true);
  let currentTime = $state(0);
  let duration = $state(0);
  let failed = $state(false);

  function toggle() {
    if (!audio) return;
    if (audio.paused) audio.play();
    else audio.pause();
  }
</script>

<div class="flex h-full w-full flex-col items-center justify-between gap-[6%]">
  <audio
    bind:this={audio}
    bind:paused
    bind:currentTime
    bind:duration
    onplay={(e) => player.claim(e.currentTarget)}
    onerror={() => (failed = true)}
    src={mediaSrc(item)}
    preload="metadata"
  ></audio>

  <div class="relative aspect-square h-[80%]">
    <div class={["record absolute inset-0 rounded-full", !paused && "spinning"]}></div>
    <div
      class="absolute inset-[33%] flex items-center justify-center rounded-full"
      style:background={coverColor(item.id)}
    >
      {#if failed}
        <Icon name="x" size={large ? 28 : 18} class="text-white/70" />
      {:else}
        <button
          onclick={toggle}
          aria-label={paused ? "Tocar" : "Pausar"}
          class="flex size-full items-center justify-center rounded-full text-white/90 hover:text-white"
        >
          <Icon name={paused ? "play" : "pause"} size={large ? 32 : 20} class={paused ? "translate-x-[8%]" : ""} />
        </button>
      {/if}
    </div>
  </div>

  {#if failed}
    <p class="text-xs text-wall/70">Áudio indisponível</p>
  {:else}
    <div class="flex w-full items-center gap-2 text-[11px] text-wall/70 tabular-nums">
      <span>{formatTime(currentTime)}</span>
      <input
        type="range"
        min="0"
        max={duration || 0}
        step="0.1"
        bind:value={currentTime}
        aria-label="Posição do áudio"
        class="h-1 min-w-0 flex-1 accent-wall"
      />
      <span>{formatTime(duration)}</span>
    </div>
  {/if}
</div>

<style>
  .record {
    /* The conic sheen is what makes the turning visible on concentric grooves. */
    background:
      conic-gradient(
        from 20deg,
        transparent 0 8%,
        rgb(255 255 255 / 0.08) 12%,
        transparent 18% 50%,
        rgb(255 255 255 / 0.08) 62%,
        transparent 68%
      ),
      radial-gradient(circle, transparent 31%, rgb(255 255 255 / 0.06) 32%, transparent 33%),
      repeating-radial-gradient(circle, #1c1f1e 0 1.5px, #262a28 1.5px 3px);
    box-shadow:
      inset 0 0 0 2px #121413,
      0 6px 14px -6px rgb(0 0 0 / 0.5);
  }

  .spinning {
    animation: spin 3.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
