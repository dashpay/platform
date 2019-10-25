const logger = require('../../src/logger');

const JSONStorage = {
  createInstance: () => ({
    setItem: (key, item) => logger.info('JSONStorage#setItem', { key, item }),
    getItem: (key) => logger('JSONStorage#getItem', key),
  }),
};
module.exports = JSONStorage;
