<script lang="ts">
  import { gallery } from "$lib/state/gallery.svelte";
  import { isHttpUrl, progressFraction, progressLabel } from "$lib/media";
  import Icon from "./Icon.svelte";
  import ProgressBar from "./ProgressBar.svelte";

  let url = $state("");
  let valid = $derived(isHttpUrl(url));
  let fileInput = $state<HTMLInputElement>();

  let busyLabel = $derived.by(() => {
    const importing = gallery.importing;
    if (!importing) return progressLabel(gallery.addProgress);
    const label = progressLabel(gallery.addProgress, "Enviando");
    return importing.total > 1 ? `${label} (${importing.current}/${importing.total})` : label;
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!valid || gallery.adding) return;
    if (await gallery.add(url.trim())) url = "";
  }

  function onchange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const files = [...(input.files ?? [])];
    input.value = "";
    gallery.importFiles(files);
  }
</script>

<form onsubmit={submit} class="relative flex w-full gap-2">
  <label class="relative flex-1">
    <span class="sr-only">Link da mídia</span>
    <input
      bind:value={url}
      type="url"
      placeholder="Cole o link de uma imagem, áudio, vídeo ou página (YouTube…)"
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
      {busyLabel}
    {:else}
      Adicionar
    {/if}
  </button>
  <button
    type="button"
    onclick={() => fileInput?.click()}
    disabled={gallery.adding}
    title="Adicionar arquivos deste computador"
    class="flex h-10 items-center gap-2 rounded-sm border border-wall-line px-3 text-ink hover:border-ink-muted disabled:cursor-not-allowed disabled:opacity-35"
  >
    <Icon name="upload" size={16} />
    Arquivos
  </button>
  <input
    bind:this={fileInput}
    {onchange}
    type="file"
    multiple
    accept="image/*,audio/*,video/*"
    class="hidden"
    aria-hidden="true"
    tabindex="-1"
  />

  {#if gallery.adding}
    <div class="absolute inset-x-0 -bottom-2">
      <ProgressBar fraction={progressFraction(gallery.addProgress)} label="Progresso do envio" />
    </div>
  {/if}
</form>
