// ESM loader for Mocha to handle .mjs files and WASM imports
export async function resolve(specifier, context, defaultResolve) {
  return defaultResolve(specifier, context);
}

export async function load(url, context, defaultLoad) {
  return defaultLoad(url, context);
}