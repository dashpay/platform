import globals from 'globals';

/**
 * Mocha test files ESLint configuration.
 * Provides test-specific globals and rule relaxations.
 */
export const mochaTestConfig = {
  files: [
    '**/test/**/*.js',
    '**/test/**/*.ts',
    '**/tests/**/*.js',
    '**/tests/**/*.ts',
    '**/*.spec.js',
    '**/*.spec.ts',
    '**/test-d/**/*.ts',
  ],
  languageOptions: {
    globals: {
      ...globals.mocha,
      expect: 'readonly',
    },
  },
  rules: {
    // Allow dev dependencies in tests
    'import-x/no-extraneous-dependencies': 'off',
    // dirty-chai uses property assertions
    'no-unused-expressions': 'off',
    '@typescript-eslint/no-unused-expressions': 'off',
  },
};

export default mochaTestConfig;
