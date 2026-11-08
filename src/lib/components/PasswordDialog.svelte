<script lang="ts" module>
  export type PasswordMode = "create" | "unlock" | "change";
</script>

<script lang="ts">
  import { gallery } from "$lib/state/gallery.svelte";
  import Icon from "./Icon.svelte";

  let { mode, onclose }: { mode: PasswordMode; onclose: () => void } = $props();

  const MIN_LENGTH = 4;

  const copy = $derived({
    create: {
      title: "Crie uma senha",
      body: "Você vai usar essa senha para ver as pastas ocultas. Não há como recuperá-la se você esquecer.",
      submit: "Criar senha",
    },
    unlock: {
      title: "Pastas ocultas",
      body: "Digite a senha para ver as pastas ocultas.",
      submit: "Desbloquear",
    },
    change: {
      title: "Trocar senha",
      body: "Digite a senha atual e escolha uma nova.",
      submit: "Trocar senha",
    },
  }[mode]);

  let current = $state("");
  let password = $state("");
  let repeat = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let passwordInput = $state<HTMLInputElement>();

  let choosing = $derived(mode !== "unlock");

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = null;

    if (choosing) {
      if (password.length < MIN_LENGTH) return (error = `A senha precisa ter pelo menos ${MIN_LENGTH} caracteres.`);
      if (password !== repeat) return (error = "As senhas não são iguais.");
    }

    busy = true;
    error =
      mode === "unlock"
        ? await gallery.unlock(password)
        : await gallery.setPassword(mode === "change" ? current : null, password);
    busy = false;

    if (!error) return onclose();
    if (mode === "unlock") {
      password = "";
      passwordInput?.focus();
    }
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && !busy && onclose()} />

<div class="fixed inset-0 z-50 flex items-center justify-center bg-wall-deep/80 p-6">
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="password-title"
    class="w-full max-w-sm rounded-sm bg-mat p-6 text-wall-deep shadow-2xl"
  >
    <form onsubmit={submit}>
      <div class="flex items-center gap-2">
        <Icon name="lock" size={18} />
        <h2 id="password-title" class="font-serif text-2xl">{copy.title}</h2>
      </div>
      <p class="mt-2 text-sm text-wall/80">{copy.body}</p>

      <div class="mt-5 flex flex-col gap-3">
        {#if mode === "change"}
          <label class="flex flex-col gap-1 text-sm">
            Senha atual
            <input bind:value={current} use:focusOnMount type="password" autocomplete="current-password" class="field" />
          </label>
        {/if}

        <label class="flex flex-col gap-1 text-sm">
          {mode === "change" ? "Nova senha" : "Senha"}
          {#if mode === "change"}
            <input bind:value={password} type="password" autocomplete="new-password" class="field" />
          {:else}
            <input
              bind:this={passwordInput}
              bind:value={password}
              use:focusOnMount
              type="password"
              autocomplete={mode === "create" ? "new-password" : "current-password"}
              class="field"
            />
          {/if}
        </label>

        {#if choosing}
          <label class="flex flex-col gap-1 text-sm">
            Repita a senha
            <input bind:value={repeat} type="password" autocomplete="new-password" class="field" />
          </label>
        {/if}
      </div>

      {#if error}
        <p role="alert" class="mt-3 text-sm text-alert">{error}</p>
      {/if}

      <div class="mt-6 flex justify-end gap-2">
        <button type="button" onclick={onclose} disabled={busy} class="px-3 py-1.5 text-sm hover:underline">
          Cancelar
        </button>
        <button
          type="submit"
          disabled={busy || !password}
          class="flex items-center gap-2 rounded-sm bg-wall-deep px-4 py-1.5 text-sm font-medium text-mat disabled:opacity-40"
        >
          {#if busy}<Icon name="loader" size={14} class="animate-spin" />{/if}
          {copy.submit}
        </button>
      </div>
    </form>
  </div>
</div>

<style>
  .field {
    height: 2.25rem;
    border: 1px solid rgb(34 48 43 / 0.25);
    border-radius: 2px;
    background: white;
    padding: 0 0.6rem;
    color: var(--color-wall-deep);
  }

  .field:focus {
    outline: none;
    border-color: var(--color-wall);
    box-shadow: 0 0 0 2px var(--color-brass);
  }
</style>
