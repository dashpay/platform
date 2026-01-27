import baseConfig from '../../eslint/base.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';
import wasmConfig from '../../eslint/wasm.mjs';

export default [
  ...baseConfig,
  {
    ...wasmConfig,
  },
  mochaTestConfig,
  {
    ignores: ['dist/**', 'pkg/**', 'node_modules/**'],
  },
];
