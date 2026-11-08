<script lang="ts" module>
  import type { MediaKind } from "$lib/types";

  export const kindLabel: Record<MediaKind, string> = {
    image: "Imagem",
    audio: "Áudio",
    video: "Vídeo",
    unknown: "Link",
  };
</script>

<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { displayTitle, hostOf } from "$lib/media";
  import { gallery } from "$lib/state/gallery.svelte";
  import type { GalleryItem } from "$lib/types";
  import Icon from "./Icon.svelte";
  import RecordArt from "./RecordArt.svelte";

  let { item, onopen }: { item: GalleryItem; onopen: () => void } = $props();

  let imageFailed = $state(false);
  let editing = $state(false);
  let draft = $state("");
  let copied = $state(false);
  let confirmingDelete = $state(false);
  let moving = $state(false);

  function moveTo(folderId: number | null) {
    moving = false;
    if (folderId !== item.folderId) gallery.move(item.id, folderId);
  }

  function startEditing() {
    draft = displayTitle(item);
    editing = true;
  }

  function commitEdit() {
    if (!editing) return;
    editing = false;
    if (draft.trim() !== displayTitle(item)) gallery.rename(item.id, draft);
  }

  async function copy() {
    await navigator.clipboard.writeText(item.url);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function remove() {
    if (confirmingDelete) gallery.remove(item.id);
    else confirmingDelete = true;
  }

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<figure class="group m-0 min-w-0" onmouseleave={() => {
    confirmingDelete = false;
    moving = false;
  }}>
  <div class="mat relative aspect-square bg-mat p-[10%]">
    {#if item.kind === "audio"}
      <RecordArt {item} />
    {:else}
      <button onclick={onopen} class="flex size-full items-center justify-center" aria-label="Ver {displayTitle(item)}">
        {#if item.kind === "image" && !imageFailed}
          <img
            src={item.url}
            alt=""
            loading="lazy"
            draggable="false"
            onerror={() => (imageFailed = true)}
            class="artwork max-h-full max-w-full object-contain"
          />
        {:else if item.kind === "video"}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video src={item.url} preload="metadata" muted class="artwork max-h-full max-w-full object-contain"></video>
          <span class="absolute flex size-11 items-center justify-center rounded-full bg-wall-deep/80 text-mat">
            <Icon name="play" size={18} class="translate-x-px" />
          </span>
        {:else}
          <span class="flex flex-col items-center gap-2 text-center text-wall/60">
            <Icon name={imageFailed ? "image" : "link"} size={28} />
            <span class="text-xs">{imageFailed ? "Imagem indisponível" : hostOf(item.url)}</span>
          </span>
        {/if}
      </button>
    {/if}

    <div
      class={[
        "absolute top-2 right-2 flex items-center rounded-sm bg-mat text-wall/70 shadow-sm transition-opacity",
        confirmingDelete || moving ? "opacity-100" : "opacity-0 group-focus-within:opacity-100 group-hover:opacity-100",
      ]}
    >
      {#if confirmingDelete}
        <button onclick={remove} class="rounded-sm bg-brass px-2 py-0.5 text-xs font-medium text-wall-deep">
          Remover
        </button>
        <button onclick={() => (confirmingDelete = false)} class="p-1 hover:text-wall-deep" aria-label="Cancelar">
          <Icon name="x" size={15} />
        </button>
      {:else}
        <button onclick={startEditing} class="p-1 hover:text-wall-deep" aria-label="Renomear" title="Renomear">
          <Icon name="pencil" size={15} />
        </button>
        <button onclick={copy} class="p-1 hover:text-wall-deep" aria-label="Copiar link" title="Copiar link">
          <Icon name={copied ? "check" : "copy"} size={15} />
        </button>
        <button onclick={() => openUrl(item.url)} class="p-1 hover:text-wall-deep" aria-label="Abrir no navegador" title="Abrir no navegador">
          <Icon name="external" size={15} />
        </button>
        <button
          onclick={() => (moving = !moving)}
          class="p-1 hover:text-wall-deep"
          aria-label="Mover para pasta"
          aria-expanded={moving}
          title="Mover para pasta"
        >
          <Icon name="folder" size={15} />
        </button>
        <button onclick={remove} class="p-1 hover:text-wall-deep" aria-label="Remover" title="Remover">
          <Icon name="trash" size={15} />
        </button>
      {/if}

      {#if moving}
        <ul class="absolute top-full right-0 z-10 mt-1 w-48 rounded-sm border border-wall-line bg-wall-deep py-1 text-ink shadow-xl">
          {#each [{ id: null, name: "Sem pasta", hidden: false }, ...gallery.folders] as folder (folder.id)}
            <li>
              <button
                onclick={() => moveTo(folder.id)}
                class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-wall"
              >
                <span class="min-w-0 flex-1 truncate">{folder.name}</span>
                {#if folder.hidden}<Icon name="eyeOff" size={13} class="text-ink-muted" />{/if}
                {#if folder.id === item.folderId}<Icon name="check" size={14} class="text-brass" />{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>

  <figcaption class="mt-3 min-w-0">
    {#if editing}
      <input
        bind:value={draft}
        use:focusSelect
        onblur={commitEdit}
        onkeydown={(e) => {
          if (e.key === "Enter") commitEdit();
          if (e.key === "Escape") editing = false;
        }}
        aria-label="Nome da peça"
        class="w-full border-b border-brass bg-transparent font-serif text-base text-ink italic focus:outline-none"
      />
    {:else}
      <button onclick={onopen} class="block max-w-full truncate text-left font-serif text-base text-ink italic hover:underline">
        {displayTitle(item)}
      </button>
    {/if}
    <p class="truncate text-xs text-ink-muted">{kindLabel[item.kind]}, {hostOf(item.url)}</p>
  </figcaption>
</figure>

<style>
  /* The mat hangs off the wall; the artwork sits slightly recessed in it. */
  .mat {
    box-shadow:
      0 1px 2px rgb(0 0 0 / 0.3),
      0 16px 28px -14px rgb(0 0 0 / 0.6);
  }

  .artwork {
    box-shadow: 0 0 0 1px rgb(0 0 0 / 0.08), 0 2px 6px rgb(0 0 0 / 0.18);
  }
</style>
