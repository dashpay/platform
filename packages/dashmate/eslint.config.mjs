import baseConfig from '../../eslint/base.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';

export default [
  ...baseConfig,
  {
    files: ['**/*.js'],
    rules: {
      // Require .js extension for imports (ESM)
      'import-x/extensions': ['error', 'ignorePackages', { js: 'always' }],
    },
  },
  mochaTestConfig,
  {
    ignores: ['dist/**', 'node_modules/**'],
  },
];
