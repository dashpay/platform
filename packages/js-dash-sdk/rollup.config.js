import typescript from '@rollup/plugin-typescript';
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import json from '@rollup/plugin-json';
import wasm from '@rollup/plugin-wasm';
import dts from 'rollup-plugin-dts';

const external = ['@dashevo/wasm-dpp', 'eventemitter3'];

const plugins = [
  wasm({
    targetEnv: 'auto-inline'
  }),
  json(),
  resolve({
    browser: true,
    preferBuiltins: false
  }),
  commonjs(),
  typescript({
    tsconfig: './tsconfig.json',
    declaration: false
  })
];

// Main bundle configuration
const mainConfig = {
  input: 'src/index.ts',
  output: [
    {
      file: 'dist/index.js',
      format: 'cjs',
      exports: 'named'
    },
    {
      file: 'dist/index.esm.js',
      format: 'es'
    }
  ],
  external,
  plugins
};

// Individual module configurations for tree-shaking
const modules = ['core', 'identities', 'contracts', 'documents', 'names', 'wallet'];

const moduleConfigs = modules.flatMap(module => {
  const inputPath = module === 'core' 
    ? 'src/core/index.ts' 
    : `src/modules/${module}/index.ts`;
  
  const outputBase = module === 'core'
    ? 'dist/core'
    : `dist/modules/${module}`;

  return [
    {
      input: inputPath,
      output: [
        {
          file: `${outputBase}/index.js`,
          format: 'cjs',
          exports: 'named'
        },
        {
          file: `${outputBase}/index.esm.js`,
          format: 'es'
        }
      ],
      external,
      plugins
    }
  ];
});

// Type definitions bundle
const dtsConfig = {
  input: 'src/index.ts',
  output: {
    file: 'dist/index.d.ts',
    format: 'es'
  },
  plugins: [dts()]
};

export default [mainConfig, ...moduleConfigs, dtsConfig];