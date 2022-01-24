// get(chainlock, HashOrHeight)
// set(header, hashOrHeight)
// length
const LRU = require("lru-cache")

const options = {
  max: 500,
  maxAge: 1000 * 60 * 60,
  length: function (n, key) {
    return n * 2 + key.length
  },
  dispose: function (key, n) {
    n.close()
  },
}

const cache = new LRU(options)

module.exports = {
  get: (key) => {
    return cache.get(key)
  },
  set: (key, value) => {
    return cache.set(key, value)
  }
}
