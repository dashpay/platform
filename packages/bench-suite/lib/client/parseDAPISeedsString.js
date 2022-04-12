/**
 * @param {string} seedsString
 * @returns {{host: string, httpPort: string, grpcPort: string}[]}
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
