// Baixa o yt-dlp para src-tauri/binaries, onde o app o procura em dev.
// O ffmpeg (para juntar vídeo e áudio) é usado do PATH: `winget install Gyan.FFmpeg`.
import { execSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const triple = execSync("rustc -vV").toString().match(/host: (\S+)/)[1];
const exe = triple.includes("windows") ? ".exe" : "";
const asset = triple.includes("windows")
  ? "yt-dlp.exe"
  : triple.includes("apple")
    ? "yt-dlp_macos"
    : "yt-dlp_linux";

const dir = join("src-tauri", "binaries");
mkdirSync(dir, { recursive: true });
console.log(`Baixando yt-dlp (${asset})...`);
const res = await fetch(`https://github.com/yt-dlp/yt-dlp/releases/latest/download/${asset}`);
if (!res.ok) throw new Error(`Falha ao baixar yt-dlp: HTTP ${res.status}`);
writeFileSync(join(dir, `yt-dlp-${triple}${exe}`), Buffer.from(await res.arrayBuffer()), { mode: 0o755 });
console.log("Pronto.");
