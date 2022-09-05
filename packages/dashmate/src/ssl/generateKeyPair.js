const forge = require('node-forge');

/**
 * @typedef {generateKeyPair}
 * @param {number} bits
 * @return {Promise<{privateKey: string, publicKey: string}>}
 */
async function generateKeyPair(bits = 2048) {
  const keys = forge.pki.rsa.generateKeyPair(bits);
  return {
    publicKey: forge.pki.publicKeyToPem(keys.publicKey),
    privateKey: forge.pki.privateKeyToPem(keys.privateKey),
  };
}

module.exports = generateKeyPair;
