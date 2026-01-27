module.exports = {
  root: true,
  env: {
    node: true,
    mocha: true,
  },
  overrides: [
    {
      files: ['**/*.ts'],
      extends: [
        'airbnb-base',
        'plugin:@typescript-eslint/recommended',
      ],
      parser: '@typescript-eslint/parser',
      parserOptions: {
        project: ['./tsconfig.tests.json'],
      },
      plugins: ['@typescript-eslint'],
      rules: {
        'import/no-extraneous-dependencies': 'off',
        'no-await-in-loop': 'off',
        'import/extensions': 'off',
        'max-len': 'off',
        'no-underscore-dangle': 'off',
        'no-bitwise': 'off',
        'no-restricted-syntax': 'off',
        'class-methods-use-this': 'off',
        'import/prefer-default-export': 'off',
        'import/no-unresolved': 'off',
        '@typescript-eslint/no-unused-vars': 'warn',
        curly: ['error', 'all'],
      },
      globals: {
        expect: true,
      },
    },
  ],
  ignorePatterns: ['karma', '*.cjs'],
};
