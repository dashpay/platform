import { configs, plugins } from 'eslint-config-airbnb-extended';
import globals from 'globals';

/**
 * Base ESLint configuration for all packages.
 * Contains common rules and overrides used across the monorepo.
 */
export const baseConfig = [
  // Register the plugins required by airbnb-extended
  // Each plugin export is a config object with nested plugins
  plugins.stylistic,
  plugins.importX,
  plugins.node,
  ...configs.base.recommended,
  {
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: 'module',
      },
      globals: {
        ...globals.node,
        BigInt: 'readonly',
      },
    },
    rules: {
      // Common overrides from existing configs
      'no-plusplus': 'off',
      'eol-last': ['error', 'always'],
      'class-methods-use-this': 'off',
      curly: ['warn', 'all'],
      'no-await-in-loop': 'off',
      'no-continue': 'off',
      // Custom restricted syntax - remove ForOfStatement restriction from airbnb
      'no-restricted-syntax': [
        'error',
        {
          selector: 'LabeledStatement',
          message: 'Labels are a form of GOTO; using them makes code confusing.',
        },
        {
          selector: 'WithStatement',
          message: '`with` is disallowed in strict mode.',
        },
      ],
      // Disable import-x rules that don't work well in monorepo
      'import-x/no-relative-packages': 'off',
      'import-x/extensions': 'off',
      'import-x/no-named-as-default': 'off',
      // Increase line length limit
      '@stylistic/max-len': ['warn', { code: 120, ignoreStrings: true, ignoreTemplateLiterals: true, ignoreUrls: true, ignoreRegExpLiterals: true, ignoreComments: true }],
      // Disable @stylistic rules that weren't enforced before
      '@stylistic/indent': 'off',
      '@stylistic/brace-style': 'off',
      '@stylistic/max-statements-per-line': 'off',
      '@stylistic/operator-linebreak': 'off',
      '@stylistic/member-delimiter-style': 'off',
      '@stylistic/type-annotation-spacing': 'off',
      '@stylistic/arrow-parens': 'off',
      '@stylistic/arrow-spacing': 'off',
      '@stylistic/semi': 'off',
      '@stylistic/comma-spacing': 'off',
      '@stylistic/comma-dangle': 'off',
      '@stylistic/quote-props': 'off',
      '@stylistic/quotes': 'off',
      '@stylistic/lines-between-class-members': 'off',
      '@stylistic/object-curly-newline': 'off',
      '@stylistic/no-mixed-operators': 'off',
      // Disable other rules that weren't enforced before
      'prefer-arrow-callback': 'off',
      'prefer-object-has-own': 'off',
      'no-new': 'off',
      'no-restricted-exports': 'off',
      // Disable import-x rules that cause too many warnings
      'import-x/newline-after-import': 'off',
      'import-x/no-useless-path-segments': 'off',
      'import-x/no-rename-default': 'off',
      'import-x/namespace': 'off',
      'import-x/prefer-default-export': 'off',
    },
  },
];

export default baseConfig;
