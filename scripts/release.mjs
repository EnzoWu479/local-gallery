// Publica uma versão a partir desta máquina, sem o GitHub Actions.
//
//   pnpm release [patch|minor|major]   (padrão: patch)
//   pnpm release --dry-run             só compila e monta os arquivos, sem commit nem publicação
//
// Sobe a versão do package.json, compila o instalador assinado, faz o commit
// "chore: versão X.Y.Z", envia a main e cria a release vX.Y.Z com o instalador,
// a assinatura e o latest.json que o auto update do app lê.
//
// A chave privada vem de TAURI_SIGNING_PRIVATE_KEY (caminho ou conteúdo) ou,
// sem ela, de ~/.tauri/local-gallery.key. Se a chave tiver senha, defina
// TAURI_SIGNING_PRIVATE_KEY_PASSWORD.
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const args = process.argv.slice(2);
const dryRun = args.includes("--dry-run");
const bump = args.find((a) => !a.startsWith("--")) ?? "patch";
if (!["patch", "minor", "major"].includes(bump)) fail(`Tipo de versão inválido: ${bump}. Use patch, minor ou major.`);

function fail(message) {
  console.error(`\n✗ ${message}`);
  process.exit(1);
}

function run(command, commandArgs, options = {}) {
  const result = spawnSync(command, commandArgs, {
    stdio: options.capture ? ["inherit", "pipe", options.allowFailure ? "pipe" : "inherit"] : "inherit",
    encoding: "utf8",
    // pnpm is a .cmd shim on Windows and needs the shell; everything else runs
    // without it so arguments with spaces pass through untouched.
    shell: process.platform === "win32" && command === "pnpm",
    env: options.env ?? process.env,
  });
  if (result.status !== 0) {
    if (options.allowFailure) return null;
    fail(`Falhou: ${command} ${commandArgs.join(" ")}`);
  }
  return options.capture ? result.stdout.trim() : "";
}

const quiet = (command, commandArgs) => run(command, commandArgs, { capture: true });
const readVersion = () => JSON.parse(readFileSync("package.json", "utf8")).version;

// 1. Conferências antes de mexer em qualquer coisa.
const keyFromEnv = process.env.TAURI_SIGNING_PRIVATE_KEY;
const keyPath = join(homedir(), ".tauri", "local-gallery.key");
if (!keyFromEnv && !existsSync(keyPath)) fail(`Chave de assinatura não encontrada em ${keyPath}.`);

if (!dryRun) {
  if (quiet("git", ["status", "--porcelain"])) fail("Há alterações não commitadas. Faça o commit ou descarte antes.");
  if (quiet("git", ["branch", "--show-current"]) !== "main") fail("Publique a partir da branch main.");
  run("git", ["fetch", "--quiet", "origin", "main"]);
  if (quiet("git", ["rev-list", "--count", "HEAD..origin/main"]) !== "0")
    fail("A origin/main tem commits que você não tem. Rode git pull antes.");
  quiet("gh", ["auth", "status"]);
}

const repo = quiet("gh", ["repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"]);

// 2. Versão nova (no dry run, compila a versão atual sem mudar nada).
const previous = readVersion();
if (!dryRun) run("pnpm", ["version", bump, "--no-git-tag-version"], { capture: true });
const version = readVersion();
const tag = `v${version}`;
if (!dryRun && run("gh", ["release", "view", tag], { capture: true, allowFailure: true }) !== null) {
  run("git", ["checkout", "--", "package.json"]);
  fail(`A release ${tag} já existe.`);
}
console.log(dryRun ? `\nCompilando ${version} (dry run)` : `\nVersão ${previous} → ${version}`);

// 3. Build assinado. Se falhar, a versão volta ao que era.
const env = {
  ...process.env,
  TAURI_SIGNING_PRIVATE_KEY: keyFromEnv ?? keyPath,
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ?? "",
};
if (run("pnpm", ["tauri", "build"], { env, allowFailure: true }) === null) {
  if (!dryRun) run("git", ["checkout", "--", "package.json"]);
  fail(dryRun ? "O build falhou." : "O build falhou; a versão do package.json foi restaurada.");
}

// 4. Arquivos da release. O GitHub troca espaços por pontos nos nomes, então
// eles já saem com pontos para a URL do latest.json bater.
const bundleDir = join("src-tauri", "target", "release", "bundle", "nsis");
const builtName = `Local Gallery_${version}_x64-setup.exe`;
const assetName = builtName.replaceAll(" ", ".");
const installer = join(bundleDir, builtName);
if (!existsSync(installer) || !existsSync(`${installer}.sig`))
  fail(`Instalador ou assinatura não encontrados em ${bundleDir}.`);

const outDir = join("src-tauri", "target", "release", "publish");
rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });
copyFileSync(installer, join(outDir, assetName));
copyFileSync(`${installer}.sig`, join(outDir, `${assetName}.sig`));

const notes = "Baixe o `Local.Gallery_*_x64-setup.exe` abaixo. Quem já tem o app recebe a atualização ao abrir.";
const platform = {
  signature: readFileSync(`${installer}.sig`, "utf8").trim(),
  url: `https://github.com/${repo}/releases/download/${tag}/${assetName}`,
};
const latest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: { "windows-x86_64": platform, "windows-x86_64-nsis": platform },
};
writeFileSync(join(outDir, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`);

const files = [assetName, `${assetName}.sig`, "latest.json"].map((f) => join(outDir, f));
if (dryRun) {
  console.log(`\n✓ Dry run pronto. Arquivos em ${outDir}:`);
  for (const f of files) console.log(`  ${f}`);
  process.exit(0);
}

// 5. Commit, push e release.
run("git", ["commit", "--quiet", "-m", `chore: versão ${version}`, "--", "package.json"]);
run("git", ["push", "--quiet", "origin", "main"]);
run("gh", [
  "release", "create", tag, ...files,
  "--target", quiet("git", ["rev-parse", "HEAD"]),
  "--title", `Local Gallery ${tag}`,
  "--notes", notes,
]);
console.log(`\n✓ ${tag} publicada: https://github.com/${repo}/releases/tag/${tag}`);
