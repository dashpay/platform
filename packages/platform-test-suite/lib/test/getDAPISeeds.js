function getDAPISeeds() {
  return process.env.DAPI_SEED
    .split(',')
    .map((seed) => {
      const [host, httpPort, grpcPort, ssl] = seed.split(':');

      return {
        host,
        httpPort,
        grpcPort,
        protocol: ssl === 'no-ssl' ? 'http' : 'https',
        selfSigned: ssl === 'self-signed',
      };
    });
}

module.exports = getDAPISeeds;
