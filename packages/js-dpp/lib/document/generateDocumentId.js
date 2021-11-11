const hashModule = require('../util/hash');

/**
 * Generates document ID
 *
 * @param {Buffer} contractId
 * @param {Buffer} ownerId
 * @param {string} type
 * @param {Buffer} entropy
 * @returns {Buffer}
 */
function generateDocumentId(contractId, ownerId, type, entropy) {
  const { hash } = hashModule;

  return hash(Buffer.concat([
    contractId,
    ownerId,
    Buffer.from(type),
    entropy,
  ]));
}

module.exports = generateDocumentId;
