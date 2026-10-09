import tenderdashSeeds from '../../configs/defaults/tenderdashSeeds.js';
import seedSetHash from './seedSetHash.js';

/**
 * Find obsolete stock lists without modifying configuration or performing I/O.
 * @param {Object} configs
 * @returns {Array} config names and replacement seeds
 */
export default function getDefaultSeedUpdates(configs) {
  return Object.entries(configs).flatMap(([name, options]) => {
    if (!['mainnet', 'testnet'].includes(options.network)) {
      return [];
    }
    const seeds = options.platform?.drive?.tenderdash?.p2p?.seeds;
    if (!Array.isArray(seeds) || seeds.some(seed => !seed || typeof seed !== 'object')) {
      return [];
    }
    const defaults = tenderdashSeeds[options.network];
    const hash = seedSetHash(seeds);
    if (hash === seedSetHash(defaults.seeds) || !defaults.previousSeedSetHashes.includes(hash)) {
      return [];
    }
    return [[name, structuredClone(defaults.seeds)]];
  });
}
