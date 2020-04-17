const bs58 = require('bs58');
const hash = require('../util/hash');

/**
 * Generate data contract id based on owner id and entropy
 *
 * @param {string} ownerId
 * @param {string} entropy
 *
 * @return {string}
 */
function generateDataContractId(ownerId, entropy) {
  return bs58.encode(
    hash(
      Buffer.concat([
        bs58.decode(ownerId),
        bs58.decode(entropy),
      ]),
    ),
  );
}

module.exports = generateDataContractId;
