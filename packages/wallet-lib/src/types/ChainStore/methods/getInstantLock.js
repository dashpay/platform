function getInstantLock(transactionHash) {
  return this.state.instantLocks.get(transactionHash);
}

module.exports = getInstantLock;
