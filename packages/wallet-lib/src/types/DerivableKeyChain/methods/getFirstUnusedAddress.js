function getFirstUnusedAddress() {
  const allUnused = this.getIssuedPaths()
    .filter((path) => path.isUsed === false);

  const firstUnused = allUnused.slice(0, 1)[0];

  return {
    path: firstUnused.path,
    address: firstUnused.address.toString(),
  };
}
module.exports = getFirstUnusedAddress;
