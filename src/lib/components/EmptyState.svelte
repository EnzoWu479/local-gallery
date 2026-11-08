<script lang="ts">
  let { variant, query = "" }: { variant: "empty-wall" | "empty-folder" | "no-match"; query?: string } = $props();
</script>

<div class="flex items-start gap-8 py-4">
  <div class="aspect-square w-40 shrink-0 border border-dashed border-wall-line"></div>
  <div class="max-w-sm pt-4">
    {#if variant === "no-match"}
      <h2 class="font-serif text-2xl text-ink">Nenhuma peça encontrada</h2>
      <p class="mt-2 text-ink-muted">
        {#if query}
          Nada com “{query}” no nome ou no link aqui. Tente outra busca.
        {:else}
          Não há peças desse tipo aqui. Escolha outro filtro para ver as demais.
        {/if}
      </p>
    {:else if variant === "empty-folder"}
      <h2 class="font-serif text-2xl text-ink">Esta pasta está vazia</h2>
      <p class="mt-2 text-ink-muted">
        Links que você adicionar com esta pasta aberta entram nela. Para trazer peças que já existem, use “Mover
        para pasta” em cada uma.
      </p>
    {:else}
      <h2 class="font-serif text-2xl text-ink">A parede está vazia</h2>
      <p class="mt-2 text-ink-muted">
        Cole o link de uma imagem, áudio ou vídeo no campo acima. Você também pode colar com Ctrl+V em qualquer
        lugar da janela ou arrastar um link do navegador para cá.
      </p>
    {/if}
  </div>
</div>
