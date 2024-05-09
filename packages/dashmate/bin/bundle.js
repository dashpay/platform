const { build } = await import('esbuild');
await build({
  bundle: true,
  entryPoints: ['./src/**/*.ts'],
  // Mark shelljs as an external dependency. Plugin-plugins v5 removes the shelljs dependency so we can remove
  // this once that's been released.
  external: ['shelljs'],
  format: 'esm',
  inject: ['./bin/cjs-shims.js'],
  loader: { '.node': 'copy' },
  outdir: './dist',
  platform: 'node',
  plugins: [],
  splitting: true,
  treeShaking: true,
});
