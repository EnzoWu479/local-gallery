# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Releases e auto update

Cada push na `main` roda `.github/workflows/release.yml`, que gera o instalador
do Windows e o guarda como artifact do workflow. Quando a versão do
`package.json` ainda não tem release, o workflow também publica a release
`v<versão>` com o instalador, a assinatura e o `latest.json`.

Ao abrir, o app lê `latest.json` da release mais recente (`plugins.updater`
em `src-tauri/tauri.conf.json`). Se houver versão nova, ele mostra um aviso.
Ao confirmar, o app baixa o instalador, confere a assinatura, instala e reabre.

Para publicar uma versão:

```bash
pnpm version patch --no-git-tag-version
git commit -am "chore: versão X.Y.Z"
git push
```

A versão do app vem do `package.json`, porque `tauri.conf.json` aponta para ele.

### Chave de assinatura

O updater só instala pacotes assinados pela chave cuja parte pública está em
`tauri.conf.json`. A chave privada fica em `~/.tauri/local-gallery.key` e não
vai para o repositório. No GitHub, ela fica em
**Settings → Secrets and variables → Actions**, no secret
`TAURI_SIGNING_PRIVATE_KEY`, com o conteúdo do arquivo. Sem esse secret, o
build falha. Se a chave se perder, as versões instaladas não aceitam mais
atualizações.
