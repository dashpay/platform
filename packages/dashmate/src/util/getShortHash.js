const crypto = require('crypto');

/**
 *
 * @param {string} hashString
 * @returns {string}
 */
function getShortHash(hashString) {
  return crypto.createHash('sha256').update(hashString).digest('hex').substring(0, 8);
}

module.exports = getShortHash;
