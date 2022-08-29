const { URL } = require('url');

/**
 * Remove everything except protocol
 *
 * @param {string} hostname
 *
 * @returns {string}
 */
function stripProtocol(hostname) {
  const url = new URL(hostname);
  return url.protocol;
}

module.exports = stripProtocol;
