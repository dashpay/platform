const LRU = require('lru-cache');
const config = require('../../../config');

const options = {
  max: config.lru.maxSize,
  maxAge: config.lru.maxAge,
  length: (n) => n.length,
};

const cache = new LRU(options);

module.exports = {
  get: (key) => cache.get(key),
  set: (key, value) => {
    cache.set(key, value);
  },
  purge: () => cache.reset(),
};
