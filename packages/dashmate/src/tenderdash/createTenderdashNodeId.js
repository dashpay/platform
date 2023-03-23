const crypto = require('crypto');

/**
 * @typedef createTenderdashNodeId
 * @param {string} nodeKey
 * @returns {string}
 */
function createTenderdashNodeId(nodeKey) {
  const nodeKeyBuffer = Buffer.from(nodeKey, 'base64');

  const publicKey = nodeKeyBuffer.slice(32);

  return crypto.createHash('sha256')
    .update(publicKey)
    .digest('hex')
    .slice(0, 40);
}

module.exports = createTenderdashNodeId;
