import baseConfig from '../../eslint/base.mjs';
import typescriptConfig from '../../eslint/typescript.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';
import wasmConfig from '../../eslint/wasm.mjs';

export default [
  ...baseConfig,
  ...typescriptConfig,
  {
    files: ['tests/**/*.ts'],
    rules: {
      ...wasmConfig.rules,
      'import-x/extensions': ['error', 'ignorePackages'],
    },
  },
  mochaTestConfig,
  {
    ignores: ['dist/**', 'pkg/**', '*.d.ts', 'tests/karma/**', 'node_modules/**'],
  },
];
