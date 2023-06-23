// A set of rules to easen misuse in the existing codebase
// We should fix these warnigns when possible
const warnRules = {
  'import/prefer-default-export': 'warn',
  'no-param-reassign': 'warn',
  'import/no-cycle': 'warn',
  'import/no-named-default': 'warn',
  'import/no-named-as-default': 'warn',
};

// A set of common rules applicable to both JS and TS files
const commonRules = {
  'no-await-in-loop': 'off',
};

module.exports = {
  extends: [
    'airbnb-base',
  ],
  root: true,
  env: {
    node: true,
    mocha: true,
  },
  rules: {
    ...warnRules,
    ...commonRules,
  },
  overrides: [
    // TypeScript general rules
    {
      files: [
        '**/*.ts',
      ],
      extends: [
        'airbnb-base',
        'airbnb-typescript/base',
      ],
      parserOptions: {
        project: ['./tsconfig.json'],
      },
      rules: {
        '@typescript-eslint/return-await': 'warn',
        ...warnRules,
        ...commonRules,
      },
    },
    // TypeScript tests
    {
      files: [
        'src/**/*.spec.ts',
        'src/test/**/*.ts',
        'tests/**/*.ts',
      ],
      rules: {
        // Ignore dirty-chai errors
        '@typescript-eslint/no-unused-expressions': 0,
        // Ignore require('dev-dependency') errors for tests and mocks
        'import/no-extraneous-dependencies': 0,
      },
      parserOptions: {
        project: ['./tsconfig.mocha.json'],
      },
    },
    // Typings tests
    {
      files: [
        'test-d/**/*.ts',
      ],
      parserOptions: {
        project: ['./tsconfig.tsd.json'],
      },
    },
    // JS tests
    {
      files: [
        'src/test/**/*.js',
        'tests/**/*.js',
        '*.config.js',
      ],
      rules: {
        // Ignore dirty-chai errors
        'no-unused-expressions': 0,
        // Ignore require('dev-dependency') errors for tests and mocks
        'import/no-extraneous-dependencies': 0,
      },
    },
  ],
  ignorePatterns: [
    // TODO: why do we have d.ts files in typescript project at all?
    '*.d.ts',
    'build',
    'dist',
  ],
};
