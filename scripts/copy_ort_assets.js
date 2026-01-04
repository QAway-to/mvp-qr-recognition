const fs = require('fs');
const path = require('path');

function copyFile(src, dst) {
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  fs.copyFileSync(src, dst);
}

function main() {
  const repoRoot = path.join(__dirname, '..');
  const srcDir = path.join(repoRoot, 'node_modules', 'onnxruntime-web', 'dist');
  const dstDir = path.join(repoRoot, 'public', 'pkg');

  if (!fs.existsSync(srcDir)) {
    console.warn('[postinstall] onnxruntime-web dist not found:', srcDir);
    return;
  }

  fs.mkdirSync(dstDir, { recursive: true });

  const entries = fs.readdirSync(srcDir, { withFileTypes: true });
  const candidates = entries
    .filter((e) => e.isFile())
    .map((e) => e.name)
    // Keep only onnxruntime-web assets, don't touch our qr_wasm.* artifacts.
    .filter((name) => {
      if (!name.startsWith('ort')) return false;
      return (
        name.endsWith('.js') ||
        name.endsWith('.mjs') ||
        name.endsWith('.wasm') ||
        name.endsWith('.map') ||
        name.endsWith('.json')
      );
    });

  let copied = 0;
  for (const name of candidates) {
    const src = path.join(srcDir, name);
    const dst = path.join(dstDir, name);
    try {
      copyFile(src, dst);
      copied++;
    } catch (e) {
      console.warn('[postinstall] failed to copy', name, e?.message || e);
    }
  }

  console.log(`[postinstall] Copied ${copied} onnxruntime-web asset(s) to public/pkg`);
}

main();


