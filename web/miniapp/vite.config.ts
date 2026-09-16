import { defineConfig } from 'vitest/config';
import solid from '@solidjs/vite-plugin';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import UnoCSS from 'unocss/vite';

export default defineConfig({
  // Turnkey client mode: no index.html and no mount file — the plugin
  // generates the entries around src/App.tsx, wrapped in src/Document.tsx
  // (or a built-in shell). `vite build` prerenders the shell into
  // dist/client/index.html and emits a purely static dist/client.
  plugins: [
    tanstackRouter({
      target: 'solid',
      autoCodeSplitting: true,
    }),

    UnoCSS(),

    solid({
      start: true,
      diagnostics: true,
    }),
  ],
  server: {
    port: 3000,
  },
  test: {
    environment: 'jsdom',
    globals: false,
    setupFiles: ['./vitest-setup.ts'],
    // if you have few tests, try commenting this
    // out to improve performance:
    isolate: false,
  },
  build: {
    target: 'esnext',
    // Keep images as asset files instead of inlining them into the JS bundle.
    assetsInlineLimit: 0,
  },
});
