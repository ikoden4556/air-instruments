import basicSsl from '@vitejs/plugin-basic-ssl';
import { defineConfig, type Plugin } from 'vite';
import { cacheVersion, listFiles, precacheList, publicContents, serviceWorker } from './scripts/precache.mjs';

/** Emits sw.js, pre-caching everything the build and public/ contain. */
function precache(): Plugin {
  return {
    name: 'air-precache',
    apply: 'build',
    generateBundle(_options, bundle) {
      const publicFiles = listFiles('public');
      const files = precacheList(Object.keys(bundle), publicFiles);
      const contents = [
        ...Object.values(bundle).map((b) => (b.type === 'chunk' ? b.code : b.source)),
        ...publicContents('public', publicFiles),
      ].map((c) => (typeof c === 'string' ? c : Buffer.from(c)));
      this.emitFile({ type: 'asset', fileName: 'sw.js', source: serviceWorker(files, cacheVersion(contents)) });
    },
  };
}

// Phones only allow camera access over HTTPS (or on localhost), so the dev server uses a
// self-signed cert. Reach it from the phone over USB: adb reverse tcp:5173 tcp:5173, then
// https://localhost:5173. HTTP=1 serves plain http, fine for localhost-only checks.
// Relative base so the build works from a GitHub Pages sub-path.
export default defineConfig({
  base: './',
  plugins: [...(process.env.HTTP ? [] : [basicSsl()]), precache()],
});
