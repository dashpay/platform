import crypto from 'crypto';

/**
 *
 * @param {string} hashString
 * @returns {string}
 */
export function getShortHash(hashString) {
  return crypto.createHash('sha256').update(hashString).digest('hex').substring(0, 8);
}
