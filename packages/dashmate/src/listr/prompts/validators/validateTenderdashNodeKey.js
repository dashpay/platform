import crypto from 'node:crypto'

/**
 * @param {string} value
 * @returns {boolean|string}
 */
export function validateTenderdashNodeKey(value) {
  if (value.length < 1) {
    return 'should not be empty';
  }

  const nodeKey = Buffer.from(value, 'base64');

  if (nodeKey.length !== 64) {
    return 'invalid format';
  }

  const privateKey = nodeKey.slice(0, 32);
  const privateKeyDer = Buffer.concat([
    Buffer.from('302e020100300506032b657004220420', 'hex'), // Static value
    privateKey,
  ]);

  const derivedPublicKeyDer = crypto.createPublicKey({
    format: 'der',
    type: 'pkcs8',
    key: privateKeyDer,
  }).export({
    format: 'der',
    type: 'spki',
  });

  const publicKey = nodeKey.slice(32);
  const publicKeyDer = Buffer.concat([
    Buffer.from('302a300506032b6570032100', 'hex'), // Static value
    publicKey,
  ]);

  if (!derivedPublicKeyDer.equals(publicKeyDer)) {
    return 'malformed key';
  }

  return true;
}
