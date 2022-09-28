const EventEmitter = require('events');
const BlsSignatures = require('bls-signatures');

const eventNames = {
  LOADED_EVENT: 'LOADED',
};

const events = new EventEmitter();
let isLoading = false;
let instance = null;

function compileWasmModule() {
  isLoading = true;
  return BlsSignatures().then((loadedInstance) => {
    instance = loadedInstance;
    isLoading = false;
    events.emit(eventNames.LOADED_EVENT);
  });
}

const bls = {
  /**
   * Compiles BlsSignature instance if it wasn't compiled yet
   * and returns module instance
   * @return {Promise<BlsSignatures>}
   */
  getInstance() {
    return new Promise((resolve) => {
      if (instance) {
        resolve(instance);

        return;
      }

      if (isLoading) {
        events.once(eventNames.LOADED_EVENT, () => {
          resolve(instance);
        });
      } else {
        compileWasmModule().then(() => {
          resolve(instance);
        });
      }
    });
  },
};

module.exports = bls;
