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
    '@typescript-eslint/no-explicit-any': 'off',
  },
  ignorePatterns: [
    '*.d.ts',
  ],
};
