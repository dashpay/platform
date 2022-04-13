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
  const storageDump = JSON.stringify(this.storage.store);

  if (dumpOptions.log) {
    // Add a linebreak to the log message for the ease of copying of the
    // truncated log from the browser consoles
    // (the text from the buffer then can be directly pasted to the JSON parser)
    logger.info('Dumping wallet storage\n', storageDump);
  }

  return storageDump;
}

module.exports = dumpStorage;
