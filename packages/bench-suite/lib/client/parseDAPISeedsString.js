/**
 * @param {string} seedsString
 * @returns {{host: string, port: string}[]}
 */
function parseDAPISeedsString(seedsString) {
  return seedsString
    .split(',')
    .map((seed) => {
      const [host, port] = seed.split(':');

      return {
        host,
        port,
      };
    });
}

module.exports = parseDAPISeedsString;
