import baseConfig from '../../eslint/base.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';

export default [
  ...baseConfig,
  mochaTestConfig,
  {
    rules: {
      // Test suite allows dev dependencies everywhere
      'import/no-extraneous-dependencies': 'off',
      // Require await in async functions
      'require-await': 'error',
    },
  },
  {
    ignores: ['dist/**', 'node_modules/**'],
  },
];
