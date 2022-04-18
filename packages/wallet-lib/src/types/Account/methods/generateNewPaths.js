/**
 * Marks addresses as used and generates new ones if needed
 * @param {string[]} addresses
 */
function generateNewPaths(addresses) {
  let issuedPaths = [];
  const keyChains = this.keyChainStore.getKeyChains();

  addresses.forEach((address) => {
    keyChains.forEach((keyChain) => {
      const keyChainIssuedPaths = keyChain.markAddressAsUsed(address);
      if (keyChainIssuedPaths.length > 0) {
        issuedPaths = issuedPaths.concat(keyChainIssuedPaths);
      }
    });
  });

  return issuedPaths;
}

module.exports = generateNewPaths;
