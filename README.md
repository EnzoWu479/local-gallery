# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Releases e auto update

As versões são compiladas nesta máquina e só os arquivos sobem para o GitHub.
Com a árvore de trabalho limpa, na `main`:

```bash
pnpm release          # patch; use `pnpm release minor` ou `major` se preferir
```

O script (`scripts/release.mjs`) sobe a versão do `package.json`, compila o
instalador assinado, faz o commit `chore: versão X.Y.Z`, envia a `main` e cria a
release `vX.Y.Z` com o instalador, a assinatura e o `latest.json`. Se o build
falhar, a versão volta ao que era. Para só compilar e conferir os arquivos, sem
commit nem publicação, use `pnpm release --dry-run`.

Ao abrir, o app lê `latest.json` da release mais recente (`plugins.updater`
em `src-tauri/tauri.conf.json`). Se houver versão nova, ele mostra um aviso.
Ao confirmar, o app baixa o instalador, confere a assinatura, instala e reabre.

O workflow `.github/workflows/release.yml` continua como reserva: ele só roda
quando disparado à mão em **Actions → Build e release → Run workflow**.

A versão do app vem do `package.json`, porque `tauri.conf.json` aponta para ele.

### Chave de assinatura

O updater só instala pacotes assinados pela chave cuja parte pública está em
`tauri.conf.json`. A chave privada fica em `~/.tauri/local-gallery.key` e não
vai para o repositório; o `pnpm release` a usa de lá (ou de
`TAURI_SIGNING_PRIVATE_KEY`, e da senha em `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
se houver). Para o workflow de reserva, ela fica em
**Settings → Secrets and variables → Actions**, no secret
`TAURI_SIGNING_PRIVATE_KEY`, com o conteúdo do arquivo. Se a chave se perder, as versões instaladas não aceitam mais
atualizações.
