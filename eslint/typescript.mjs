import { configs, plugins } from 'eslint-config-airbnb-extended';

/**
 * TypeScript ESLint configuration.
 * Extends base config with TypeScript-specific rules and relaxations.
 */
export const typescriptConfig = [
  // Register the TypeScript plugin
  plugins.typescriptEslint,
  ...configs.base.typescript,
  {
    files: ['**/*.ts', '**/*.tsx'],
    rules: {
      // Disable base rules that conflict with TypeScript versions
      'no-return-await': 'off',
      '@typescript-eslint/return-await': 'warn',
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': 'warn',
      'no-shadow': 'off',
      '@typescript-eslint/no-shadow': 'error',

      // Relax strict TypeScript rules for backward compatibility
      '@typescript-eslint/no-require-imports': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/ban-ts-comment': 'off',
      '@typescript-eslint/no-unsafe-function-type': 'off',
      '@typescript-eslint/no-this-alias': 'off',
      '@typescript-eslint/no-namespace': 'off',
      '@typescript-eslint/no-use-before-define': 'off',
      'no-use-before-define': 'off',

      // Disable new @typescript-eslint rules not previously enforced
      '@typescript-eslint/consistent-indexed-object-style': 'off',
      '@typescript-eslint/consistent-type-definitions': 'off',
      '@typescript-eslint/no-inferrable-types': 'off',
      '@typescript-eslint/array-type': 'off',
      '@typescript-eslint/prefer-function-type': 'off',

      // Import resolution handled by TypeScript
      'import-x/extensions': 'off',
      'import-x/no-unresolved': 'off',

      // Disable @stylistic rules for TypeScript files (not previously enforced)
      '@stylistic/member-delimiter-style': 'off',
      '@stylistic/type-annotation-spacing': 'off',
    },
  },
];

export default typescriptConfig;
