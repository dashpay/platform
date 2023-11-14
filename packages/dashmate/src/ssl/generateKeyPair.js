import forge from 'node-forge';

/**
 * @typedef {generateKeyPair}
 * @param {number} [bits=2048]
 * @return {Promise<{privateKey: string, publicKey: string}>}
 */
export async function generateKeyPair(bits = 2048) {
  const keys = forge.pki.rsa.generateKeyPair(bits);
  return {
    publicKey: forge.pki.publicKeyToPem(keys.publicKey),
    privateKey: forge.pki.privateKeyToPem(keys.privateKey),
  };
}
