<script lang="ts">
  import { gallery } from "$lib/state/gallery.svelte";
  import { isHttpUrl } from "$lib/media";
  import Icon from "./Icon.svelte";

  let url = $state("");
  let valid = $derived(isHttpUrl(url));

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!valid || gallery.adding) return;
    if (await gallery.add(url.trim())) url = "";
  }
</script>

<form onsubmit={submit} class="flex w-full gap-2">
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
    class="flex h-10 items-center gap-2 rounded-sm bg-mat px-4 font-medium text-wall-deep transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-35"
  >
    {#if gallery.adding}
      <Icon name="loader" size={16} class="animate-spin" />
      Adicionando
    {:else}
      Adicionar
    {/if}
  </button>
</form>
