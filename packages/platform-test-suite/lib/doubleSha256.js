const crypto = require('crypto');

module.exports = function doubleSha246(payload) {
  // The implementation of hash in Node.js is stateful and requires separate objects
  const hasher1 = crypto.createHash('sha256');
  const firstHash = hasher1.update(payload).digest();

  const hasher2 = crypto.createHash('sha256');
  return hasher2.update(firstHash).digest('hex');
};
