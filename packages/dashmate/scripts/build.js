#!/usr/bin/env node

// eslint-disable-next-line import/no-extraneous-dependencies
import * as esbuild from 'esbuild';
// eslint-disable-next-line import/no-extraneous-dependencies
import fileloc from 'esbuild-plugin-fileloc';

await esbuild.build({
  entryPoints: ['./src/index.js'],
  bundle: true,
  outdir: './dist',
  platform: 'node',
  target: 'node20',
  format: 'esm',
  inject: ['./scripts/build/shim.js'],
  plugins: [fileloc.filelocPlugin()],
  external: ['ejs', '@dashevo/bls', '@dashevo/wasm-dpp'],
  treeShaking: true,
  loader: {
    '.node': 'copy',
  },
});
