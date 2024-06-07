const { Block } = require('@dashevo/dashcore-lib');
const logger = require('../logger');

/**
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 * @return {emitBlockEventToFilterCollection}
 */
function emitBlockEventToFilterCollectionFactory(bloomFilterEmitterCollection) {
  /**
   * Emit `block` event to bloom filter collection
   *
   * @param {Buffer} rawBlock
   */
  function emitBlockEventToFilterCollection(rawBlock) {
    const block = new Block(rawBlock);

    logger.debug(`block ${block.hash} received`);

    bloomFilterEmitterCollection.emit('block', block);
  }

  return emitBlockEventToFilterCollection;
}

module.exports = emitBlockEventToFilterCollectionFactory;
