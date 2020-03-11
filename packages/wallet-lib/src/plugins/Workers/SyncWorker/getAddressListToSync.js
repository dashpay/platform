module.exports = function getAddressListToSync() {
  const addressList = [];

  const { addresses } = this.storage.getStore().wallets[this.walletId];
  Object.keys(addresses).forEach((walletType) => {
    const walletAddresses = addresses[walletType];
    const walletPaths = Object.keys(walletAddresses);
    if (walletPaths.length > 0) {
      walletPaths.forEach((path) => {
        const address = walletAddresses[path];
        addressList.push(address);
      });
    }
  });
  return addressList;
};
