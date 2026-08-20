import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { VitePWA } from 'vite-plugin-pwa';
import { execSync } from 'child_process';

let gitCommit = 'dev';
try {
  gitCommit = process.env.GITHUB_SHA
    ? process.env.GITHUB_SHA.substring(0, 7)
    : execSync('git rev-parse --short HEAD').toString().trim();
} catch {}

export default defineConfig({
  base: process.env.GITHUB_PAGES ? '/bourbakimesh/' : '/',
  define: {
    __APP_BUILD_TIME__: JSON.stringify(new Date().toISOString()),
    __APP_VERSION__: JSON.stringify(gitCommit),
    __APP_GIT_COMMIT__: JSON.stringify(gitCommit),
  },
  server: {
    port: 5173,
    allowedHosts: true,
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8000',
        changeOrigin: true,
        configure: (proxy) => {
          proxy.on('error', (_err, _req, res) => {
            // Gracefully return 503 when backend daemon is offline so frontend uses local PWA fallbacks
            if (res && 'writeHead' in res && typeof res.writeHead === 'function' && !res.headersSent) {
              res.writeHead(503, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({ status: 'offline', error: 'Backend daemon offline (using local PWA fallbacks)' }));
            }
          });
        },
      },
      '/ws': {
        target: 'ws://127.0.0.1:8000',
        ws: true,
        configure: (proxy) => {
          proxy.on('error', (_err) => {
            // Silently swallow WebSocket proxy connection errors when daemon is offline
          });
        },
      },
    },
  },
  preview: {
    port: 5173,
    allowedHosts: true,
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.ico', 'robots.txt', 'apple-touch-icon.png', 'data/ledger_snapshot.json'],
      workbox: {
        cleanupOutdatedCaches: true,
        maximumFileSizeToCacheInBytes: 35 * 1024 * 1024,
        globPatterns: ['**/*.{js,css,html,ico,png,svg,json,wasm}'],
        clientsClaim: true,
        skipWaiting: true,
      },
      manifest: {
        name: 'BourbakiMesh Prover',
        short_name: 'BourbakiMesh',
        description: 'Decentralized Game-Semantic Theorem Prover & Proof DAG Explorer',
        theme_color: '#0f172a',
        background_color: '#0f172a',
        display: 'standalone',
        icons: [
          {
            src: 'pwa-192x192.png',
            sizes: '192x192',
            type: 'image/png',
          },
          {
            src: 'pwa-512x512.png',
            sizes: '512x512',
            type: 'image/png',
          },
        ],
      },
    }),
  ],
});
