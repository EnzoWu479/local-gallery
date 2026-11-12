<script lang="ts">
  // Checks GitHub for a newer version once at launch and offers it. Installing
  // downloads the signed installer, runs it and the app reopens on its own.
  import { onMount } from "svelte";
  import * as api from "$lib/api";
  import type { UpdateInfo, UpdateProgress } from "$lib/types";
  import Icon from "./Icon.svelte";
  import ProgressBar from "./ProgressBar.svelte";

  let update = $state<UpdateInfo | null>(null);
  let dismissed = $state(false);
  let installing = $state(false);
  let progress = $state<UpdateProgress | null>(null);
  let error = $state<string | null>(null);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    api.onUpdateProgress((p) => (progress = p)).then((u) => (unlisten = u));
    // Offline or no release yet: no banner, just a note in the console.
    api.checkUpdate().then(
      (found) => (update = found),
      (e) => console.warn(e),
    );
    return () => unlisten?.();
  });

  async function install() {
    installing = true;
    error = null;
    try {
      await api.installUpdate();
    } catch (e) {
      error = String(e);
      installing = false;
      progress = null;
    }
  }

  const fraction = $derived(progress?.total ? progress.received / progress.total : null);
  const mb = (bytes: number) => (bytes / 1024 / 1024).toFixed(1);
</script>

{#if update && !dismissed}
  <div
    role="status"
    class="fixed right-6 bottom-6 z-50 flex w-80 flex-col gap-3 rounded-sm bg-mat px-4 py-3 text-wall-deep shadow-xl"
  >
    <div class="flex items-start gap-3">
      <Icon name="download" size={16} class="mt-0.5 shrink-0" />
      <div class="flex-1">
        <p class="font-medium">Versão {update.version} disponível</p>
        <p class="text-sm opacity-70">
          {#if installing}
            {progress ? `Baixando ${mb(progress.received)}${progress.total ? ` de ${mb(progress.total)}` : ""} MB` : "Preparando"}
            · o app reabre sozinho
          {:else}
            Você está na {update.currentVersion}.
          {/if}
        </p>
      </div>
      {#if !installing}
        <button onclick={() => (dismissed = true)} aria-label="Agora não" class="opacity-60 hover:opacity-100">
          <Icon name="x" size={16} />
        </button>
      {/if}
    </div>

    {#if installing}
      <ProgressBar {fraction} label="Baixando a atualização" />
    {:else}
      {#if error}
        <p class="text-sm text-alert">{error}</p>
      {/if}
      <button
        onclick={install}
        class="self-start rounded-sm bg-wall-deep px-3 py-1.5 text-sm text-mat hover:opacity-90"
      >
        Atualizar e reiniciar
      </button>
    {/if}
  </div>
{/if}
