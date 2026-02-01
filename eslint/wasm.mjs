/**
 * WASM-specific ESLint configuration.
 * Relaxes rules that conflict with WASM bindings and FFI patterns.
 */
export const wasmConfig = {
  rules: {
    // WASM bindings use __type, _wbg_ptr naming conventions
    'no-underscore-dangle': 'off',
    // Legitimate use in hex conversion and buffer operations
    'no-bitwise': 'off',
    // for-of loops are acceptable in modern TypeScript
    'no-restricted-syntax': 'off',
    // Extended line length for WASM binding code
    'max-len': [
      'error',
      {
        code: 140,
        ignoreStrings: true,
        ignoreTemplateLiterals: true,
      },
    ],
  },
};

export default wasmConfig;
