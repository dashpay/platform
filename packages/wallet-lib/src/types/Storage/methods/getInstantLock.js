/**
 *
 * @param {string} transactionHash
 * @return {InstantLock}
 */
function getInstantLock(transactionHash) {
  return this.store.instantLocks[transactionHash];
}

module.exports = getInstantLock;
