const { Transaction, InstantLock } = require('@dashevo/dashcore-lib');

/**
 * @param {BloomFilterEmitterCollection} bloomFilterEmitterCollection
 * @return {emitInstantLockToFilterCollection}
 */
function emitInstantLockToFilterCollectionFactory(bloomFilterEmitterCollection) {
  /**
   * Emit `islock` event to bloom filter collection
   *
   * @param {Buffer} rawTransactionLock
   */
  function emitInstantLockToFilterCollection(rawTransactionLock) {
    const transaction = new Transaction().fromBuffer(rawTransactionLock);
    const txBuffer = transaction.toBuffer();

    const txLockBuffer = rawTransactionLock.slice(txBuffer.length, rawTransactionLock.length);

    const instantLock = new InstantLock(txLockBuffer);

    bloomFilterEmitterCollection.emit('instantLock', {
      transaction,
      instantLock,
    });
  }

  return emitInstantLockToFilterCollection;
}

module.exports = emitInstantLockToFilterCollectionFactory;
