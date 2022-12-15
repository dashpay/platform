import * as dpp_module from "../lib/dpp";
// import { inspect } from 'util';

export default function (dppModule: typeof dpp_module) {
    // As Identifier is now exported from WASM, it can no longer
    // patched the same way as JS implementation did. This module
    // adds everything that's currently missing from Identifier that can't
    // be implemented in Rust directly.
    const { Identifier } = dppModule;

    //@ts-ignore
    Object.setPrototypeOf(Identifier.prototype, Buffer.prototype);

    Identifier.prototype.valueOf = function() {
        return Buffer.from(this.inner());
    }

    // @ts-ignore
    Identifier.prototype.encodeCBOR = function encodeCBOR(encoder) {
        // @ts-ignore
        encoder.pushAny(this.valueOf());

        return true;
    };

    // @ts-ignore
    Identifier.prototype.inspect = function(...args) {
        return this.valueOf().inspect(...args);
    }

    // TODO: THIS MAKES BUFFERS PRINTABLE IN NODE.JS, BUT FOR THIS TO WORK
    //  target: node has to be specified and utils have to be included __without__ using
    //  polyfills. This in turn will make web version not work. The code is as follows:
    //  console.log("custom:", inspect.custom);
    //  //@ts-ignore
    //  Identifier.prototype[inspect.custom] = Identifier.prototype.inspect;
}