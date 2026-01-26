module.exports = {
  root: true,
  env: {
    node: true,
    mocha: true,
  },
  overrides: [
    // TypeScript tests
    {
      files: ['tests/**/*.ts'],
      extends: [
        'airbnb-base',
        'plugin:@typescript-eslint/recommended',
      ],
      parser: '@typescript-eslint/parser',
      parserOptions: {
        project: ['./tests/tsconfig.json'],
      },
      plugins: ['@typescript-eslint'],
      rules: {
        // Ignore dirty-chai errors
        '@typescript-eslint/no-unused-expressions': 'off',
        // Ignore require('dev-dependency') errors for tests
        'import/no-extraneous-dependencies': 'off',
        // Allow any for test flexibility
        '@typescript-eslint/no-explicit-any': 'off',
        // Common relaxations for tests
        'no-await-in-loop': 'off',
        'import/extensions': ['error', 'ignorePackages'],
        // Increase line length limit for tests (100 is too restrictive)
        'max-len': ['error', { code: 140, ignoreStrings: true, ignoreTemplateLiterals: true }],
        // Keep disabled - WASM bindings use underscores (__type, _wbg_ptr)
        'no-underscore-dangle': 'off',
        // Keep disabled - legitimate use in hex conversion
        'no-bitwise': 'off',
        // Keep disabled - for-of loops are fine in modern TS
        'no-restricted-syntax': 'off',
      },
      settings: {
        'import/resolver': {
          node: {
            extensions: ['.js', '.ts'],
          },
        },
      },
    },
  ],
  ignorePatterns: [
    'dist',
    'pkg',
    '*.d.ts',
    'tests/karma',
  ],
};
