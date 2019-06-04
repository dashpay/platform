const { Block } = require('@dashevo/dashcore-lib');

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

    bloomFilterEmitterCollection.emit('block', block);
  }

  return emitBlockEventToFilterCollection;
}

module.exports = emitBlockEventToFilterCollectionFactory;
