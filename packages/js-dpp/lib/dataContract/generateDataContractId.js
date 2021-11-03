const hashModule = require('../util/hash');

/**
 * Generate data contract id based on owner id and entropy
 *
 * @param {Buffer} ownerId
 * @param {Buffer} entropy
 *
 * @return {Buffer}
 */
function generateDataContractId(ownerId, entropy) {
  const { hash } = hashModule;

  return hash(
    Buffer.concat([
      ownerId,
      entropy,
    ]),
  );
}

module.exports = generateDataContractId;
