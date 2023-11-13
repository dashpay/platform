import crypto from 'crypto';

/**
 * @typedef deriveTenderdashNodeId
 * @param {string} nodeKey
 * @returns {string}
 */
export function deriveTenderdashNodeId(nodeKey) {
  const nodeKeyBuffer = Buffer.from(nodeKey, 'base64');

  const publicKey = nodeKeyBuffer.slice(32);

  return crypto.createHash('sha256')
    .update(publicKey)
    .digest('hex')
    .slice(0, 40);
}
