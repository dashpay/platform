import baseConfig from '../../eslint/base.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';

export default [
  ...baseConfig,
  mochaTestConfig,
  {
    ignores: ['dist/**', 'node_modules/**'],
  },
];
