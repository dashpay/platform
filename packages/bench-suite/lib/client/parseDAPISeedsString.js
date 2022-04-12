/**
 * @param {string} seedsString
 * @returns {*}
 */
function parseDAPISeedsString(seedsString) {
  return seedsString
    .split(',')
    .map((seed) => {
      const [host, httpPort, grpcPort] = seed.split(':');

      return {
        host,
        httpPort,
        grpcPort,
      };
    });
}

module.exports = parseDAPISeedsString;
