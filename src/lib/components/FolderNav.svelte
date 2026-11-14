<script lang="ts">
  import { gallery } from "$lib/state/gallery.svelte";
  import Icon from "./Icon.svelte";
  import PasswordDialog, { type PasswordMode } from "./PasswordDialog.svelte";

  let dialog = $state<PasswordMode | null>(null);

  let creating = $state(false);
  let newName = $state("");
  let newHidden = $state(false);

  let editingId = $state<number | null>(null);
  let draft = $state("");
  let confirmingId = $state<number | null>(null);

  async function create(event: SubmitEvent) {
    event.preventDefault();
    if (!newName.trim()) return;
    if (await gallery.createFolder(newName, newHidden)) closeCreate();
  }

  function closeCreate() {
    creating = false;
    newName = "";
    newHidden = false;
  }

  function startRename(id: number, name: string) {
    confirmingId = null;
    editingId = id;
    draft = name;
  }

  async function commitRename() {
    if (editingId === null) return;
    const id = editingId;
    editingId = null;
    const folder = gallery.folders.find((f) => f.id === id);
    if (folder && draft.trim() && draft.trim() !== folder.name)
      await gallery.updateFolder(id, { name: draft });
  }

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  const rowClass = (active: boolean) => [
    "flex w-full items-center gap-2.5 border-l-2 py-1.5 pr-3 pl-4 text-left transition-colors",
    active
      ? "border-brass bg-wall text-ink"
      : "border-transparent text-ink-muted hover:bg-wall/60 hover:text-ink",
  ];
</script>

<nav aria-label="Pastas" class="flex h-full flex-col py-4">
  <button
    onclick={() => gallery.selectFolder("all")}
    class={rowClass(gallery.folder === "all")}
  >
    <Icon name="wall" size={15} class="shrink-0" />
    <span class="flex-1">Tudo</span>
  </button>

  <h2 class="mt-5 mb-1 px-[18px] text-xs text-ink-muted">Pastas</h2>

  <ul class="flex flex-col">
    {#each gallery.folders as folder (folder.id)}
      <li>
        {#if editingId === folder.id}
          <div class="py-1 pr-3 pl-4">
            <input
              bind:value={draft}
              use:focusSelect
              onblur={commitRename}
              onkeydown={(e) => {
                if (e.key === "Enter") commitRename();
                if (e.key === "Escape") editingId = null;
              }}
              aria-label="Nome da pasta"
              class="w-full border-b border-brass bg-transparent py-0.5 text-ink focus:outline-none"
            />
          </div>
        {:else}
          <div class="group/row relative">
            <button
              onclick={() => gallery.selectFolder(folder.id)}
              class={rowClass(gallery.folder === folder.id)}
            >
              <Icon
                name={folder.hidden ? "eyeOff" : "folder"}
                size={15}
                class="shrink-0"
              />
              <span class="min-w-0 flex-1 truncate">{folder.name}</span>
              <span class="text-xs text-ink-muted tabular-nums"
                >{gallery.countIn(folder.id)}</span
              >
            </button>

            <div
              class="absolute inset-y-0 right-1 hidden items-center bg-wall text-ink-muted group-focus-within/row:flex group-hover/row:flex"
            >
              <button
                onclick={() => startRename(folder.id, folder.name)}
                class="p-1 hover:text-ink"
                aria-label="Renomear pasta"
                title="Renomear"
              >
                <Icon name="pencil" size={14} />
              </button>
              {#if gallery.unlocked}
                <button
                  onclick={() =>
                    gallery.updateFolder(folder.id, { hidden: !folder.hidden })}
                  class="p-1 hover:text-ink"
                  aria-label={folder.hidden
                    ? "Deixar pasta visível"
                    : "Ocultar pasta"}
                  title={folder.hidden ? "Deixar visível" : "Ocultar"}
                >
                  <Icon name={folder.hidden ? "eye" : "eyeOff"} size={14} />
                </button>
              {/if}
              <button
                onclick={() => (confirmingId = folder.id)}
                class="p-1 hover:text-ink"
                aria-label="Remover pasta"
                title="Remover"
              >
                <Icon name="trash" size={14} />
              </button>
            </div>
          </div>
        {/if}

        {#if confirmingId === folder.id}
          <div class="mx-3 my-1 rounded-sm bg-mat p-3 text-xs text-wall-deep">
            <p>Remover “{folder.name}”? As peças dela continuam em Tudo.</p>
            <div class="mt-2 flex gap-2">
              <button
                onclick={() => {
                  confirmingId = null;
                  gallery.deleteFolder(folder.id);
                }}
                class="rounded-sm bg-wall-deep px-2 py-1 font-medium text-mat"
              >
                Remover pasta
              </button>
              <button onclick={() => (confirmingId = null)} class="px-2 py-1"
                >Cancelar</button
              >
            </div>
          </div>
        {/if}
      </li>
    {/each}
  </ul>

  {#if creating}
    <form
      onsubmit={create}
      class="mx-3 mt-2 flex flex-col gap-2 rounded-sm border border-wall-line p-3"
    >
      <input
        bind:value={newName}
        use:focusSelect
        onkeydown={(e) => e.key === "Escape" && closeCreate()}
        placeholder="Nome da pasta"
        aria-label="Nome da nova pasta"
        class="border-b border-wall-line bg-transparent py-0.5 text-ink placeholder:text-ink-muted/70 focus:border-brass focus:outline-none"
      />
      {#if gallery.unlocked}
        <label class="flex items-center gap-2 text-xs text-ink-muted">
          <input
            type="checkbox"
            bind:checked={newHidden}
            class="accent-brass"
          />
          Pasta oculta
        </label>
      {/if}
      <div class="flex gap-2">
        <button
          type="submit"
          disabled={!newName.trim()}
          class="rounded-sm bg-mat px-2.5 py-1 text-xs font-medium text-wall-deep disabled:opacity-35"
        >
          Criar pasta
        </button>
        <button
          type="button"
          onclick={closeCreate}
          class="px-2 py-1 text-xs text-ink-muted hover:text-ink"
        >
          Cancelar
        </button>
      </div>
    </form>
  {:else}
    <button
      onclick={() => (creating = true)}
      class="mt-1 flex items-center gap-2.5 py-1.5 pl-[18px] text-ink-muted hover:text-ink"
    >
      <Icon name="plus" size={15} />
      Nova pasta
    </button>
  {/if}

  <div
    class="mt-auto flex flex-col items-start gap-2 pt-6 pl-[18px] text-xs text-ink-muted"
  >
    {#if gallery.unlocked}
      <button
        onclick={() => gallery.lock()}
        class="flex items-center justify-center w-8 h-8 gap-2.5 hover:text-ink hover:bg-[#ffffff10] rounded-sm cursor-pointer transition-all"
      >
        <Icon name="lock" size={14} />
      </button>
      <button
        onclick={() => (dialog = "change")}
        class="pl-[24px] hover:text-ink hover:underline">Trocar senha</button
      >
    {:else}
      <button
        onclick={() => (dialog = gallery.hasPassword ? "unlock" : "create")}
        class="flex items-center justify-center w-8 h-8 gap-2.5 hover:text-ink hover:bg-[#ffffff10] rounded-sm cursor-pointer transition-all"
      >
        <Icon name="unlock" size={14} />
      </button>
    {/if}
  </div>
</nav>

{#if dialog}
  <PasswordDialog mode={dialog} onclose={() => (dialog = null)} />
{/if}
