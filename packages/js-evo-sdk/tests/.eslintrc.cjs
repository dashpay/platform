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
        project: ['./tsconfig.json'],
      },
      plugins: ['@typescript-eslint'],
      rules: {
        'import/no-extraneous-dependencies': 'off',
        'no-await-in-loop': 'off',
        'import/extensions': ['error', 'ignorePackages'],
        'max-len': ['error', { code: 140 }],
        'no-underscore-dangle': 'off',
        'no-bitwise': 'off',
        'no-restricted-syntax': 'off',
        'class-methods-use-this': 'off',
        'import/prefer-default-export': 'off',
        'import/no-unresolved': 'off',
        curly: ['error', 'all'],
      },
      globals: {
        expect: true,
      },
    },
  ],
  ignorePatterns: ['karma', '*.cjs'],
};
