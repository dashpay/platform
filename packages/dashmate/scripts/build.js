#!/usr/bin/env node

import * as esbuild from 'esbuild';
import fileloc from 'esbuild-plugin-fileloc';

await esbuild.build({
  entryPoints: ['./src/index.js', 'src/help.js'],
  bundle: true,
  outdir: './dist',
  platform: 'node',
  target: 'node20',
  format: 'esm',
  inject: ['./scripts/build/shim.js'],
  plugins: [fileloc.filelocPlugin()],
  // external: ['ejs'],
  treeShaking: true,
  loader: {
    '.node': 'copy',
  },
  mainFields: ['main'],
});
