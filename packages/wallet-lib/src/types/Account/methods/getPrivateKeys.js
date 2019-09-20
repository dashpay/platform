/**
 * Return all the private keys matching the PubKey Addr List
 * @param addressList<String>
 * @return {Array}<HDPrivateKey>
 */
function getPrivateKeys(addressList) {
  let addresses = [];
  let privKeys = [];
  if (addressList.constructor.name === Object.name) {
    addresses = [addressList];
  } else { addresses = addressList; }

  const { walletId } = this;
  const self = this;
  const subwallets = Object.keys(this.store.wallets[walletId].addresses);
  subwallets.forEach((subwallet) => {
    const paths = Object.keys(self.store.wallets[walletId].addresses[subwallet]);
    paths.forEach((path) => {
      const address = self.store.wallets[walletId].addresses[subwallet][path];
      if (addresses.includes(address.address)) {
        const privateKey = self.keyChain.getKeyForPath(path);
        privKeys = privKeys.concat([privateKey]);
      }
    });
  });

  return privKeys;
}
module.exports = getPrivateKeys;
