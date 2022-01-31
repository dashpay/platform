const ChainStore = require('../../ChainStore/ChainStore');

/**
 * Create when does not yet exist a chainStore
 * @param network
 * @return {boolean}
 */
const createChainStore = function createChain(network) {
  if (!this.chains.has(network.toString())) {
    this.chains.set(network.toString(), new ChainStore(network.toString()));
    return true;
  }
  return false;
};
module.exports = createChainStore;
