import tenderdashSeeds from '../../configs/defaults/tenderdashSeeds.js';
import filterTenderdashSeeds, { getTenderdashSeedNodeIds } from './filterTenderdashSeeds.js';
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
    const { seeds: currentSeeds, previousSeedSetHashes } = tenderdashSeeds[options.network];
    const hash = seedSetHash(seeds);
    if (hash === seedSetHash(currentSeeds)) {
      return [];
    }

    // The stored list stays full so a subsequent node key or ID change cannot
    // make it look like a custom list. Recover older lists tied to either ID.
    const isStock = previousSeedSetHashes.includes(hash)
      || getTenderdashSeedNodeIds(tenderdash.node).some(nodeId => (
        previousSeedSetHashes.includes(seedSetHash(seeds, nodeId))
        || hash === seedSetHash(filterTenderdashSeeds(currentSeeds, nodeId))
      ));
    return isStock ? [[name, structuredClone(currentSeeds)]] : [];
  });
}
