"use strict";
var EventEmitter = require("events");
var BlsSignatures = require("@dashevo/bls");
var eventNames = {
  LOADED_EVENT: "LOADED",
};
var events = new EventEmitter();
var isLoading = false;
var instance = null;
function compileWasmModule() {
  isLoading = true;
  return BlsSignatures().then(function (loadedInstance) {
    instance = loadedInstance;
    isLoading = false;
    events.emit(eventNames.LOADED_EVENT);
  });
}
var bls = {
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
module.exports = bls;
//# sourceMappingURL=bls.js.map
