const LRU = require('lru-cache');
const config = require('../config');

const options = {
  max: config.blockHeaders.cache.maxSize,
  maxAge: config.blockHeaders.cache.maxAge,
  length: (n) => n && n.length,
};

class BlockHeadersCache {
  constructor() {
    this.cache = new LRU(options);
  }

  get(key) {
    return this.cache.get(key);
  }

  set(key, value) {
    this.cache.set(key, value);
  }

  purge() {
    this.cache.reset();
  }
}

module.exports = BlockHeadersCache;
