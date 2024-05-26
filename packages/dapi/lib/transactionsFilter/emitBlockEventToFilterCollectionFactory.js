const logger = require('../logger');

/**
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 * @return {emitBlockEventToFilterCollection}
 */
function emitBlockEventToFilterCollectionFactory(bloomFilterEmitterCollection) {
  /**
   * Emit `block` event to bloom filter collection
   *
   * @param {Block} block
   */
  function emitBlockEventToFilterCollection(block) {
    logger.debug(`block ${block.hash} received`);

    bloomFilterEmitterCollection.emit('block', block);
  }

  return emitBlockEventToFilterCollection;
}

module.exports = emitBlockEventToFilterCollectionFactory;
