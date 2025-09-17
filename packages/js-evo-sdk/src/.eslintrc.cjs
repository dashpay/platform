module.exports = {
  parser: '@typescript-eslint/parser',
  parserOptions: {
    project: ['../tsconfig.json'],
    tsconfigRootDir: __dirname,
  },
  env: {
    es2020: true,
    browser: true,
    node: true,
  },
  extends: [
    'airbnb-base',
    'airbnb-typescript/base',
  ],
  plugins: [
    '@typescript-eslint',
  ],
  rules: {
    'import/extensions': 'off',
    'import/prefer-default-export': 'off',
    'object-curly-newline': 'off',
    'class-methods-use-this': 'off',
    'max-len': 'off',
    'no-restricted-exports': 'off',
    '@typescript-eslint/no-explicit-any': 'off',
    '@typescript-eslint/lines-between-class-members': 'off',
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_', ignoreRestSiblings: true }],
  },
  ignorePatterns: [
    '*.d.ts',
  ],
};
