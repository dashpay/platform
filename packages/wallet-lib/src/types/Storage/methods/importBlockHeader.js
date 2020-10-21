const EVENTS = require('../../../EVENTS');
/**
 * This method is used to import a blockheader in Store.
 * @param {BlockHeader} blockHeader - A Blockheader
 * @param {number} height
 */
const importBlockHeader = function importBlockHeader(blockHeader, height) {
  const self = this;
  const { store, network } = this;

  const chainStore = store.chains[network];
  const { blockHeight: currentChainHeight } = store.chains[network];

  if (!chainStore.blockHeaders[blockHeader.hash]) {
    if (height) {
      if (height > currentChainHeight) store.chains[network].blockHeight = height;
      else {
        store.chains[network].blockHeight += 1;
        self.announce(EVENTS.BLOCKHEIGHT_CHANGED, store.chains[network].blockHeight);
      }
    }
    const blockHeight = height || currentChainHeight;

    chainStore.blockHeaders[blockHeader.hash] = blockHeader;
    chainStore.mappedBlockHeaderHeights[blockHeight] = blockHeader.hash;

    self.announce(EVENTS.BLOCKHEADER, blockHeader);
  }
};
module.exports = importBlockHeader;
