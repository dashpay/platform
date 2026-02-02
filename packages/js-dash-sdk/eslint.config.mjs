import baseConfig from '../../eslint/base.mjs';
import typescriptConfig from '../../eslint/typescript.mjs';
import mochaTestConfig from '../../eslint/mocha-tests.mjs';

// Rules that need to be fixed - temporarily set to warn
const warnRules = {
  'import-x/prefer-default-export': 'warn',
  'no-param-reassign': 'warn',
  'import-x/no-cycle': 'warn',
  'import-x/no-named-default': 'warn',
  'import-x/no-named-as-default': 'warn',
};

export default [
  ...baseConfig,
  ...typescriptConfig,
  {
    files: ['**/*.ts'],
    rules: {
      ...warnRules,
    },
  },
  {
    files: ['src/**/*.spec.ts', 'src/test/**/*.ts', 'tests/**/*.ts'],
    rules: {
      '@typescript-eslint/no-unused-expressions': 'off',
      'import-x/no-extraneous-dependencies': 'off',
    },
  },
  {
    files: ['src/test/**/*.js', 'tests/**/*.js', '*.config.js', 'karma/**/*.js'],
    rules: {
      'no-unused-expressions': 'off',
      'import-x/no-extraneous-dependencies': 'off',
    },
  },
  mochaTestConfig,
  {
    ignores: ['**/*.d.ts', 'test-d/**', 'build/**', 'dist/**', 'node_modules/**'],
  },
];
