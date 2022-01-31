function getWatchedAddresses() {
  return [...this.issuedPaths.values()];
}

module.exports = getWatchedAddresses;
