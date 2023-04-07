"use strict";
var __importDefault =
  (this && this.__importDefault) ||
  function (mod) {
    return mod && mod.__esModule ? mod : { default: mod };
  };
Object.defineProperty(exports, "__esModule", { value: true });
var events_1 = __importDefault(require("events"));
var wasm_dpp_1 = __importDefault(require("@dashevo/wasm-dpp"));
var eventNames = {
  LOADED_EVENT: "LOADED",
};
var events = new events_1.default();
var isLoading = false;
var instance = null;
function compileWasmModule() {
  isLoading = true;
  return wasm_dpp_1.default().then(function (loadedInstance) {
    instance = loadedInstance;
    isLoading = false;
    events.emit(eventNames.LOADED_EVENT);
  });
}
exports.default = {
  /**
   * Compiles BlsSignature instance if it wasn't compiled yet
   * and returns module instance
   * @return {Promise<BlsSignatures>}
   */
  getInstance: function () {
    return new Promise(function (resolve) {
      if (instance) {
        resolve(instance);
        return;
      }
      if (isLoading) {
        events.once(eventNames.LOADED_EVENT, function () {
          resolve(instance);
        });
      } else {
        compileWasmModule().then(function () {
          resolve(instance);
        });
      }
    });
  },
};
//# sourceMappingURL=WasmDPP.js.map
