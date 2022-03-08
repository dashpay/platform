const { WALLET_TYPES, BIP44_LIVENET_ROOT_PATH, BIP44_TESTNET_ROOT_PATH } = require('../CONSTANTS');

function classifyAddresses(walletStore, accountIndex, walletType, network = 'testnet') {
  const externalAddressesList = [];
  const internalAddressesList = [];
  const otherAccountAddressesList = [];
  const miscAddressesList = [];

  const rootPath = (network.toString() === 'testnet')
    ? BIP44_TESTNET_ROOT_PATH
    : BIP44_LIVENET_ROOT_PATH;

  const accountsPaths = [...walletStore.state.paths.keys()];

  const isHDWallet = [
    WALLET_TYPES.HDWALLET,
    WALLET_TYPES.HDPRIVATE,
    WALLET_TYPES.HDPUBLIC].includes(walletType);

  const currentAccountPath = (isHDWallet) ? `${rootPath}/${accountIndex}'` : `m/${accountIndex}`;

  accountsPaths.forEach((accountPath) => {
    const isCurrentAccountPath = accountPath === currentAccountPath;
    const accountPaths = walletStore.getPathState(accountPath);

    Object.entries(accountPaths.addresses)
      .forEach(([path, address]) => {
        if (isCurrentAccountPath) {
          if (isHDWallet) {
            if (path.startsWith('m/0')) externalAddressesList.push(address);
            else if (path.startsWith('m/1')) internalAddressesList.push(address);
            else miscAddressesList.push(address);
          } else {
            externalAddressesList.push(address);
          }
        } else {
          otherAccountAddressesList.push(address);
        }
      });
  });

  return {
    externalAddressesList,
    internalAddressesList,
    otherAccountAddressesList,
    miscAddressesList,
  };
}
module.exports = classifyAddresses;
