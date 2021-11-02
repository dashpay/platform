const blake3Promise = require('blake3/dist/node');

// Including this file in the same file as merk segfaults the test,
// so webasm used instead
let blake3;
async function init() {
  blake3 = await blake3Promise;
}

/**
 * @param {Buffer} data
 * @return {Buffer}
 */
function hashFunction(data) {
  return blake3.hash(data);
}

module.exports = { init, hashFunction };
