const hash = require('../util/hash');

/**
 * Generate data contract id based on owner id and entropy
 *
 * @param {Buffer} ownerId
 * @param {Buffer} entropy
 *
 * @return {Buffer}
 */
function generateDataContractId(ownerId, entropy) {
  return hash(
    Buffer.concat([
      ownerId,
      entropy,
    ]),
  );
}

module.exports = generateDataContractId;
