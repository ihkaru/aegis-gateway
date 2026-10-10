import { svelte } from '@sveltejs/vite-plugin-svelte';
import { build } from 'vite';

async function run() {
  console.log('Starting Svelte 5 programmatic build via Bun...');
  await build({
    configFile: false,
    plugins: [svelte()],
    base: './',
    build: {
      outDir: 'dist',
      emptyOutDir: true,
    },
  });
  console.log('Build completed successfully!');
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});
