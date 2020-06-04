const { URL } = require('url');

/**
 * Remove everything except (hostname/ip):port pair
 *
 * @param {string} hostname
 *
 * @returns {string}
 */
function stripHostname(hostname) {
  const url = new URL(hostname);
  return url.host;
}

module.exports = stripHostname;
