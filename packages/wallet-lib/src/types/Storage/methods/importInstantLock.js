/**
 * Imports instant lock to the storage
 * @param {InstantLock} instantLock
 */
function importInstantLock(instantLock) {
  this.store.instantLocks[instantLock.txid] = instantLock;
}

module.exports = importInstantLock;
