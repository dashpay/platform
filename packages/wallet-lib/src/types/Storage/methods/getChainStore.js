function getChainStore(network) {
  return this.chains.get(network);
}
module.exports = getChainStore;
