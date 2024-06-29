#!/usr/bin/env node

import * as esbuild from 'esbuild';
import fileloc from 'esbuild-plugin-fileloc';

await esbuild.build({
  entryPoints: ['./bin/run.js', './src/commands/index.js'],
  bundle: true,
  outdir: './dist',
  platform: 'node',
  target: 'node20',
  format: 'esm',
  inject: ['./scripts/build/shim.js'],
  plugins: [fileloc.filelocPlugin()],
  loader: {
    '.node': 'file',
    // '.proto': 'file',
  },
});
