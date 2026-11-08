# Plano: guardar as mídias num S3 local (rclone)

**Status:** planejado, ainda não implementado.

Hoje a galeria guarda só o link. A imagem, o áudio ou o vídeo é carregado direto do site de origem toda vez que aparece na tela. Se o link sair do ar, a peça quebra.

A ideia é que, ao adicionar um link, o app baixe o arquivo e guarde uma cópia num S3 local servido pelo rclone. Na hora de exibir, a mídia vem **só** desse S3, nunca do link original.

## Por que rclone

O `rclone serve s3` transforma uma pasta comum num servidor compatível com S3, usando um único executável e sem Docker. Cada subpasta da pasta servida vira um bucket.

```bash
rclone serve s3 --auth-key CHAVE,SEGREDO --addr 127.0.0.1:PORTA <pasta>
```

Outras opções avaliadas:

- **SeaweedFS:** funciona no Windows, mas é mais do que precisamos.
- **RustFS:** ainda recente e pouco maduro.
- **MinIO:** a versão gratuita deixou de publicar executáveis prontos em 2025.
- **Garage:** sem versão oficial para Windows.

## Como vai funcionar

### 1. O app controla o rclone

- **Ao abrir:** o Rust inicia `rclone serve s3` como processo filho e espera a porta aceitar conexão antes de liberar o uso.
- **Endereço:** escuta só em `127.0.0.1`, numa porta livre escolhida na hora (abrir um socket na porta 0, ler a porta e fechar).
- **Credenciais:** chave e segredo aleatórios, gerados no primeiro uso e guardados na tabela `settings`.
- **Pastas:** os dados ficam em `<pasta de dados do app>/s3/`, e o bucket `gallery` é só a pasta `s3/gallery/`, criada antes de iniciar o rclone.
- **Ao fechar:** o app encerra o rclone no `RunEvent::Exit`.
- **Janela de console:** no Windows, iniciar com `CREATE_NO_WINDOW` para não abrir uma.
- **Onde achar o rclone:** primeiro procurar no `PATH`. Na distribuição, empacotar o `rclone.exe` como *sidecar* do Tauri (`externalBin`).
- **Se o rclone não existir:** o app abre mesmo assim, mas adicionar e exibir mídias mostram um erro claro.

### 2. Adicionar um link = baixar + subir

1. O Rust baixa o arquivo, numa thread separada.
2. O tipo vem da extensão ou do `Content-Type` da própria resposta do download, sem precisar de outra requisição.
3. Links que não são imagem, áudio ou vídeo (uma página HTML, por exemplo) são **recusados** com uma mensagem clara. Hoje eles entram como "Link".
4. O arquivo sobe para o S3 com uma chave aleatória (`media/<hex>`) e o `Content-Type` original.
5. Só depois o item é gravado no SQLite. Se qualquer passo falhar, nada é salvo.

**Limite de tamanho:** começar com 512 MB. Atenção: o `ureq` limita o corpo da resposta a 10 MB por padrão, então é preciso configurar o limite no `read_to_vec`. Se arquivos grandes pesarem na memória, trocar por envio em stream para o S3.

### 3. Exibir = protocolo `media://` lendo do S3

O WebView nunca fala com o S3 nem conhece as credenciais. O Rust registra um protocolo próprio com `register_asynchronous_uri_scheme_protocol("media", …)`:

- **Endereço:** no frontend, `convertFileSrc(String(item.id), "media")`, que no Windows vira `http://media.localhost/<id>`.
- **Leitura:** o handler busca o item no banco, lê o objeto no S3 e devolve os bytes com o `Content-Type` salvo.
- **Avançar e voltar em áudio e vídeo:** repassar o cabeçalho `Range` para o S3 e responder `206` com `Content-Range`. O tamanho total fica salvo no banco.
- **Pastas ocultas:** se a peça está numa pasta oculta e o cofre está bloqueado, responder `403`.

Também foram consideradas URLs pré-assinadas do S3, mas foram descartadas. Elas expõem a porta do rclone ao WebView, não respeitam o bloqueio das pastas ocultas e não há garantia de que o `rclone serve s3` as suporte.

### 4. Remover

Remover uma peça também apaga o objeto do S3.

## Mudanças no banco (migração 4)

```sql
ALTER TABLE items ADD COLUMN object_key TEXT;
ALTER TABLE items ADD COLUMN content_type TEXT;
ALTER TABLE items ADD COLUMN size INTEGER;
```

- `url` continua sendo o link de origem: serve para "Abrir no navegador" e para impedir links repetidos.
- O frontend recebe um campo `stored: boolean` (se existe `object_key`), nunca a chave em si.

## Frontend

- **Endereço da mídia:** um helper `mediaSrc(item)` em `api.ts`. No Tauri usa o protocolo `media://`; no mock do navegador continua usando `item.url`, porque `convertFileSrc` não existe fora do Tauri.
- **Componentes:** `Piece`, `RecordArt` e `Viewer` passam a usar `mediaSrc(item)` em vez de `item.url`.
- **Texto do botão:** durante o envio, "Adicionar" muda para "Baixando".
- **Peças antigas** (salvas antes desta mudança, sem `object_key`): mostrar "Mídia não salva" com o botão "Baixar agora", que chama um comando novo `store_item(id)`.
- **Mock:** os itens do `api.mock.ts` ganham `stored: true`, e o mock passa a ter o comando `store_item`.

## Novos comandos Rust

- `add_item(url, folder_id)`: passa a baixar e subir o arquivo antes de gravar.
- `store_item(id)`: baixa e sobe uma peça antiga que ainda não tem cópia.
- `delete_item(id)`: passa a apagar também o objeto.

## Dependências

- **Cliente S3:** `rust-s3`, com `default-features = false` e `tokio-rustls-tls`. Alternativas: `object_store` (com a feature `aws`) ou `aws-sdk-s3`, mais pesado para compilar.
- **Aleatoriedade:** `getrandom`, para as credenciais e as chaves dos objetos.
- **rclone:** 1.75.2 já instalado nesta máquina via `winget install Rclone.Rclone`.

## Pontos em aberto

- **Processo órfão:** se o app travar, o rclone continua rodando. Avaliar um *job object* do Windows para encerrar o processo filho junto.
- **Empacotamento:** decidir se o `rclone.exe` vai junto no instalador como sidecar (cerca de 60 MB) ou se o rclone será um pré-requisito.
- **Disco sem criptografia:** os arquivos ficam legíveis em `s3/gallery/`, inclusive os de pastas ocultas. Hoje o banco também não é criptografado.
