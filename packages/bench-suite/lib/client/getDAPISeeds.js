function getDAPISeeds() {
  return process.env.DAPI_SEED
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

module.exports = getDAPISeeds;
