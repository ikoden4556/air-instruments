// The service worker: pre-caches the whole app (code, core wasm, MediaPipe, hand model) on
// the first visit, so it then works offline. Built by the Vite plugin in vite.config.ts.
import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

/** Every file under `dir`, as forward-slash paths relative to it. */
export function listFiles(dir) {
  return readdirSync(dir, { recursive: true })
    .map((f) => join(dir, String(f)))
    .filter((f) => statSync(f).isFile())
    .map((f) => relative(dir, f).split('\\').join('/'));
}

/** What to pre-cache: the page, the built bundle and the public files, without source maps. */
export function precacheList(bundleFiles, publicFiles) {
  // index.html is emitted after this plugin runs, so it's named here.
  const files = ['./', 'index.html', ...bundleFiles, ...publicFiles].filter((f) => !f.endsWith('.map') && f !== 'sw.js');
  return [...new Set(files)].sort();
}

/** A short version id that changes whenever any cached file's content changes. */
export function cacheVersion(contents) {
  const hash = createHash('sha256');
  for (const c of contents) hash.update(c);
  return hash.digest('hex').slice(0, 12);
}

export function serviceWorker(files, version) {
  return `// Generated at build time (scripts/precache.mjs).
const CACHE = 'air-${version}';
const FILES = ${JSON.stringify(files)};

self.addEventListener('install', (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(FILES)).then(() => self.skipWaiting()));
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (e) => {
  if (e.request.method !== 'GET') return;
  e.respondWith(
    caches.match(e.request, { ignoreSearch: true }).then(
      (hit) =>
        hit ||
        fetch(e.request).catch(() =>
          e.request.mode === 'navigate' ? caches.match('./') : Response.error(),
        ),
    ),
  );
});
`;
}

/** Reads the public files' contents for the version hash. */
export function publicContents(dir, files) {
  return files.map((f) => readFileSync(join(dir, f)));
}
