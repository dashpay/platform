function getForAddress(address) {
  const searchResult = [...this.issuedPaths.entries()]
    .find(([, el]) => el.address.toString() === address.toString());

  if (!searchResult) {
    return null;
  }
  const [path] = searchResult;
  return this.getForPath(path);
}

module.exports = getForAddress;
