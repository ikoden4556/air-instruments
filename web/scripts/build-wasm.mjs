// Builds the Rust bindings to WebAssembly and copies the module to public/air.wasm.
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const crate = fileURLToPath(new URL('../../bindings/wasm/', import.meta.url));
execFileSync('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown'], {
  cwd: crate,
  stdio: 'inherit',
});
const pub = fileURLToPath(new URL('../public/', import.meta.url));
mkdirSync(pub, { recursive: true });
copyFileSync(`${crate}target/wasm32-unknown-unknown/release/air_wasm.wasm`, `${pub}air.wasm`);
