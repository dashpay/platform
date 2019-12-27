const {
  is,
  seedToHDPrivateKey,
} = require('../../../utils');

/**
 * Will set a wallet to work with a seed (HDPrivateKey)
 * fixme: Term seed is often use, but we might want to rename to fromHDPrivateKey
 * @param seed
 */
module.exports = function fromSeed(seed) {
  if (!is.seed(seed)) throw new Error('Expected a valid seed (typeof string)');
  return this.fromHDPrivateKey(seedToHDPrivateKey(seed, this.network));
};
