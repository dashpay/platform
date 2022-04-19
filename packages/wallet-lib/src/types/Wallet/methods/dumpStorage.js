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

  this.storage.wallets.forEach((wallet) => {
    storage.wallets[wallet.walletId] = wallet.state;
  });

  this.storage.chains.forEach((chain) => {
    storage.chains[chain.network] = chain.state;
  });

  const storageDump = JSON.stringify(storage, (jsonKey, jsonValue) => {
    if (jsonValue instanceof Map) {
      const object = {};

      // eslint-disable-next-line no-restricted-syntax
      for (const [key, value] of jsonValue.entries()) {
        object[key] = value;
      }

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
