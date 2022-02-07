const LRU = require('lru-cache');

const options = {
  max: 500,
  maxAge: 1000 * 60 * 60,
  length: (n) => n.length
};

const cache = new LRU(options);

module.exports = {
  get: (key) => cache.get(key),
  set: (key, value) => {
    cache.set(key, value);
  },
  purge: () => cache.reset(),
};
