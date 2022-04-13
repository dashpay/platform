function importInstantLock(instantLock) {
  this.state.instantLocks.set(instantLock.txid, instantLock);
}

module.exports = importInstantLock;
