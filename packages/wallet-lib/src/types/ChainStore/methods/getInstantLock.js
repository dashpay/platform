function getInstantLock(transactionHash) {
  return this.state.instantLocks.get(transactionHash);
}

export default getInstantLock;