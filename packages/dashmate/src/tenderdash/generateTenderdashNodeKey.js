const crypto = require('crypto');

/**
 * @typedef generateTenderdashNodeKey
 * @returns {string}
 */
function generateTenderdashNodeKey() {
  const {
    privateKey,
    publicKey,
  } = crypto.generateKeyPairSync('ed25519', {
    publicKeyEncoding: {
      type: 'spki',
      format: 'der',
    },
    privateKeyEncoding: {
      type: 'pkcs8',
      format: 'der',
    },
  });

  const nodeKey = Buffer.concat([privateKey.slice(16), publicKey.slice(12)]);

  return nodeKey.toString('base64');
}

module.exports = generateTenderdashNodeKey;
