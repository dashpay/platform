let chainLock;

module.exports = {
  updateBestChainLock: (chainlock) => {
    chainLock = chainlock;
  },
  getBestChainLock: () => chainLock,
};
