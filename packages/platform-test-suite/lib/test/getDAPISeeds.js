function getDAPISeeds() {
  return process.env.DAPI_SEED
    .split(',')
    .map((seed) => {
      const [host, httpPort, grpcPort, noSsl] = seed.split(':');

      return {
        host,
        httpPort,
        grpcPort,
        protocol: noSsl === 'no-ssl' ? 'http' : 'https',
      };
    });
}

module.exports = getDAPISeeds;
