import baseConfig from '../../eslint/base.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';

export default [
  // Global ignores
  {
    ignores: [
      '**/*.d.ts',
      'dist/**',
      'node_modules/**',
      'fixtures/**',
    ],
  },
  ...baseConfig,
  mochaTestConfig,
  {
    // Ignore spec files by default, but allow specific ones
    ignores: [
      '**/*.spec.js',
      'tests/**',
    ],
  },
  {
    // Re-include specific test files for linting
    files: [
      'tests/integration/**/*.js',
      'src/plugins/**/*.spec.js',
    ],
  },
];
