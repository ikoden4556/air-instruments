import basicSsl from '@vitejs/plugin-basic-ssl';
import { defineConfig } from 'vite';

// Phones only allow camera access over HTTPS (or on localhost), so the dev server uses a
// self-signed cert. Reach it from the phone over USB: adb reverse tcp:5173 tcp:5173, then
// https://localhost:5173. HTTP=1 serves plain http, fine for localhost-only checks.
// Relative base so the build works from a GitHub Pages sub-path.
export default defineConfig({
  base: './',
  plugins: process.env.HTTP ? [] : [basicSsl()],
});
