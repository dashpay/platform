const LRU = require('lru-cache');
const config = require('../config');

const options = {
  max: config.blockHeaders.cache.maxSize,
  maxAge: config.blockHeaders.cache.maxAge,
  length: (n) => n.length,
};

const blockHeadersCache = new LRU(options);

module.exports = {
  get: (key) => blockHeadersCache.get(key),
  set: (key, value) => {
    blockHeadersCache.set(key, value);
  },
  purge: () => blockHeadersCache.reset(),
};
