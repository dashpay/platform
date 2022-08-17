import init, * as dpp_module from './wasm/wasm_dpp.js';
import wasmBase from './wasm/wasm_dpp_bg.js';

export default async function load_dpp() {
    let bytes = Buffer.from(wasmBase, 'base64');

    if (typeof fetch !== 'undefined') {
        let blob = new Blob([bytes], { type: "application/wasm" });
        let wasmUrl = URL.createObjectURL(blob);
        await init(wasmUrl);
        return dpp_module;
    }  else {
        dpp_module.initSync(bytes);
        return dpp_module;
    }
};

