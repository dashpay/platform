function getWatchedAddresses() {
  return [...this.issuedPaths.entries()]
    .filter(([, el]) => el.isWatched === true)
    .map(([, el]) => el.address.toString());
}

export default getWatchedAddresses;