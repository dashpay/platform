const { hasProp } = require('../../../utils');

/**
 * Create when does not yet exist a chain in the store
 * @param network
 * @return {boolean}
 */
const createChain = function (network) {
  if (!hasProp(this.store.chains, network.toString())) {
    this.store.chains[network.toString()] = {
      name: network.toString(),
      blockHeaders: {},
      // Map a blockheader to it's height (used by searchBlockheader for speed up the process)
      mappedBlockHeaderHeights: {},
      blockHeight: -1,
    };
    return true;
  }
  return false;
};
module.exports = createChain;
