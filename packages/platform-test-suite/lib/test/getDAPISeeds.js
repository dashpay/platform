function getDAPISeeds() {
  return process.env.DAPI_SEED
    .split(',')
    .map((seed) => ({ service: `${seed}` }));
}

module.exports = getDAPISeeds;
