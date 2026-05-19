function importInstantLock(instantLock) {
  this.state.instantLocks.set(instantLock.txid, instantLock);
}

export default importInstantLock;