import init from '../wasm/wasm_dpp';
import * as dpp_module from './dpp';
// @ts-ignore
import wasmBase from '../wasm/wasm_dpp_bg.js';
import { Identifier, IdentifierError } from "./identifier/Identifier";

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
    } else {
      dpp_module.initSync(bytes);
      isInitialized = true;
    }

    // TODO: fix TS warning
    // @ts-ignore
    dpp_module.Identifier = Identifier;
    // @ts-ignore
    dpp_module.IdentifierError = IdentifierError;

    return dpp_module;
  }
};

