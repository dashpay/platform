import baseConfig from '../../eslint/base.mjs';
import typescriptConfig from '../../eslint/typescript.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';

export default [
  ...baseConfig,
  ...typescriptConfig,
  mochaTestConfig,
  {
    ignores: ['dist/**', '**/*.d.ts', 'node_modules/**'],
  },
];
