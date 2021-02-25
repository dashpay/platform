const { WALLET_TYPES } = require('../../../../CONSTANTS');

module.exports = function getAddressesToSync() {
  const { BIP44PATH, walletId, walletType } = this;
  const { addresses } = this.storage.getStore().wallets[walletId];

  const isHDWallet = [WALLET_TYPES.HDPUBLIC, WALLET_TYPES.HDWALLET].includes(walletType);
  // We have two cases, for privateKey based wallet, we return all address in store.
  // But for HDWallet, addresses in store can be of another account, that we filter out
  return Object.keys(addresses)
    .map((addressType) => Object.values(addresses[addressType]))
    .flatMap((addressList) => addressList)
    .filter((accountAddress) => !isHDWallet || accountAddress.path.startsWith(BIP44PATH))
    .map((filteredAccountAddress) => filteredAccountAddress.address);
};
