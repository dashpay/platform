const bs58 = require('bs58');

const hash = require('../util/hash');

/**
 * Generates document ID
 *
 * @param {string} contractId
 * @param {string} ownerId
 * @param {string} type
 * @param {string} entropy
 * @returns {string}
 */
function generateDocumentId(contractId, ownerId, type, entropy) {
  return bs58.encode(
    hash(Buffer.concat([
      bs58.decode(contractId),
      bs58.decode(ownerId),
      Buffer.from(type),
      bs58.decode(entropy),
    ])),
  );
}

module.exports = generateDocumentId;
