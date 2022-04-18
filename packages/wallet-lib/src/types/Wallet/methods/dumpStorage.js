const logger = require('../../../logger');

const defaultOptions = {
  log: false,
};

/**
 * Dumps storage on user's demand
 *
 * @param options - dumping options
 * @return {string} - Returns JSON string of the wallet store
 */
function dumpStorage(options) {
  const dumpOptions = options !== null && typeof options === 'object'
    ? Object.assign(defaultOptions, options)
    : defaultOptions;

  const storage = { chains: {}, wallets: {} };

  this.storage.wallets.forEach(([id, walletStore]) => {
    storage.wallets[id] = walletStore.state;
  });

  this.storage.chains.forEach(([id, chainStore]) => {
    storage.chains[id] = chainStore.state;
  });

  const storageDump = JSON.stringify(storage, (jsonKey, jsonValue) => {
    if (jsonValue instanceof Map) {
      const object = {};

      jsonValue.entries().forEach(([key, value]) => {
        object[key] = value;
      });

      return object;
    }

    return jsonValue;
  });

  if (dumpOptions.log) {
    // Add a linebreak to the log message for the ease of copying of the
    // truncated log from the browser consoles
    // (the text from the buffer then can be directly pasted to the JSON parser)
    logger.info('Dumping wallet storage\n', storageDump);
  }

  return storageDump;
}

module.exports = dumpStorage;
