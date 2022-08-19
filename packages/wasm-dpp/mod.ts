import init, * as dpp_module from './wasm';
// @ts-ignore
import wasmBase from './wasm/wasm_dpp_bg.js';

let isInitialized = false;

export default async function loadDpp() {
  if (isInitialized) {
    return dpp_module;
  } else {
    // @ts-ignore
    let bytes = Buffer.from(wasmBase, 'base64');

    if (typeof fetch !== 'undefined') {
      let blob = new Blob([bytes], { type: "application/wasm" });
      let wasmUrl = URL.createObjectURL(blob);
      await init(wasmUrl);
      isInitialized = true;
      return dpp_module;
    }  else {
      dpp_module.initSync(bytes);
      isInitialized = true;
      return dpp_module;
    }
  }
};

