function getWatchedAddresses() {
  return [...this.issuedPaths.values()];
}

export default getWatchedAddresses;