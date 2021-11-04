const blake3Promise = require('@dashevo/blake3/browser-async');

let blake3 = {};
/**
 * Init the blake 3 hasher
 * @returns {Promise<void>}
 */
async function initBlake3() {
  blake3 = await blake3Promise();
}

/**
 * Serialize and hash payload using blake 3
 *
 * @param {Buffer} buffer
 * @return {Buffer}
 */
function hash(buffer) {
  return Buffer.from(blake3.hash(buffer));
}

module.exports = {
  initBlake3,
  hash,
};
