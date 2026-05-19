import {
  is,
  seedToHDPrivateKey,
} from '../../../utils/index.js';

/**
 * Will set a wallet to work with a seed (HDPrivateKey)
 * fixme: Term seed is often use, but we might want to rename to fromHDPrivateKey
 * @param seed
 */
export default function fromSeed(seed, network) {
  if (!is.seed(seed)) throw new Error('Expected a valid seed (typeof string)');
  return this.fromHDPrivateKey(seedToHDPrivateKey(seed, network));
};
