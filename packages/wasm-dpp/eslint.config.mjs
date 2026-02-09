import baseConfig from '../../eslint/base.mjs';
import typescriptConfig from '../../eslint/typescript.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';
import wasmConfig from '../../eslint/wasm.mjs';

export default [
  ...baseConfig,
  ...typescriptConfig,
  {
    ...wasmConfig,
  },
  mochaTestConfig,
  {
    files: ['**/*.ts'],
    rules: {
      // wasm-dpp specific relaxations for TypeScript files
      'max-classes-per-file': 'off',
      'import-x/export': 'off',
      '@typescript-eslint/no-useless-constructor': 'off',
      'no-constructor-return': 'off',
      '@stylistic/object-curly-spacing': 'off',
      '@stylistic/no-extra-semi': 'off',
      '@stylistic/no-multiple-empty-lines': 'off',
      'prefer-const': 'off',
    },
  },
  {
    ignores: ['dist/**', 'pkg/**', 'node_modules/**', 'lib/wasm/**', 'test-d/**'],
  },
];
