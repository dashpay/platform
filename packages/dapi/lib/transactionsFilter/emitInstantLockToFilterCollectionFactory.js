const { Transaction, InstantLock } = require('@dashevo/dashcore-lib');

/**
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 * @return {testBlockEventToFilterCollection}
 */
function emitInstantLockToFilterCollectionFactory(bloomFilterEmitterCollection) {
  /**
   * Emit `islock` event to bloom filter collection
   *
   * @param {Buffer} rawTransactionLock
   */
  function testBlockEventToFilterCollection(rawTransactionLock) {
    const txBuffer = new Transaction().fromBuffer(rawTransactionLock).toBuffer();
    const txLockBuffer = rawTransactionLock.slice(txBuffer.length, rawTransactionLock.length);
    const transactionLock = new InstantLock(txLockBuffer);

    bloomFilterEmitterCollection.emit('instantLock', transactionLock);
  }

  return testBlockEventToFilterCollection;
}

module.exports = emitInstantLockToFilterCollectionFactory;
