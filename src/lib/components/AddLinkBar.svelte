<script lang="ts">
  import { gallery } from "$lib/state/gallery.svelte";
  import { isHttpUrl, progressFraction, progressLabel } from "$lib/media";
  import Icon from "./Icon.svelte";
  import ProgressBar from "./ProgressBar.svelte";

  let url = $state("");
  let valid = $derived(isHttpUrl(url));

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!valid || gallery.adding) return;
    if (await gallery.add(url.trim())) url = "";
  }
</script>

<form onsubmit={submit} class="relative flex w-full gap-2">
  <label class="relative flex-1">
    <span class="sr-only">Link da mídia</span>
    <input
      bind:value={url}
      type="url"
      placeholder="Cole o link de uma imagem, áudio ou vídeo"
      class="h-10 w-full rounded-sm border border-wall-line bg-wall-deep px-3 text-ink placeholder:text-ink-muted/70 focus:border-brass focus:outline-none"
    />
  </label>
  <button
    type="submit"
    disabled={!valid || gallery.adding}
    class="flex h-10 min-w-36 items-center justify-center gap-2 rounded-sm bg-mat px-4 font-medium text-wall-deep tabular-nums transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-35"
  >
    {#if gallery.adding}
      <Icon name="loader" size={16} class="animate-spin" />
      {progressLabel(gallery.addProgress)}
    {:else}
      Adicionar
    {/if}
  </button>

  {#if gallery.adding}
    <div class="absolute inset-x-0 -bottom-2">
      <ProgressBar fraction={progressFraction(gallery.addProgress)} label="Progresso do download" />
    </div>
  {/if}
</form>
