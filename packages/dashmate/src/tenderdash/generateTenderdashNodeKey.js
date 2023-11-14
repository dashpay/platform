import crypto from 'crypto';

/**
 * @typedef generateTenderdashNodeKey
 * @returns {string}
 */
export function generateTenderdashNodeKey() {
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
