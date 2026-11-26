import tenderdashSeeds from '../../configs/defaults/tenderdashSeeds.js';
import seedSetHash from './seedSetHash.js';

/**
 * Find stock lists to refresh without modifying configuration or performing I/O.
 * @param {Object} configs
 * @returns {Array} config names and replacement seeds
 */
export default function getDefaultSeedUpdates(configs) {
  return Object.entries(configs).flatMap(([name, options]) => {
    if (!['mainnet', 'testnet'].includes(options.network)) {
      return [];
    }
    const tenderdash = options.platform?.drive?.tenderdash;
    const seeds = tenderdash?.p2p?.seeds;
    if (!Array.isArray(seeds) || seeds.some(seed => !seed || typeof seed !== 'object')) {
      return [];
    }
    const defaults = tenderdashSeeds[options.network];
    const nodeId = tenderdash.node?.id;
    const currentSeeds = defaults.seeds.filter(seed => seed.id !== nodeId);
    const hash = seedSetHash(seeds);
    if (hash === seedSetHash(currentSeeds)) {
      return [];
    }

    const isStock = hash === seedSetHash(defaults.seeds)
      || defaults.previousSeedSetHashes.includes(hash)
      || (nodeId && defaults.previousSeedSetHashes.includes(seedSetHash(seeds, nodeId)));
    return isStock ? [[name, structuredClone(currentSeeds)]] : [];
  });
}
