const Storage = require('../../types/Storage/Storage');

const mockStorage = () => {
  const network = 'testnet';
  const walletId = `walletId_${Math.random() * 100}`;

  const storage = new Storage();
  storage.currentWalletId = walletId;
  storage.currentNetwork = network;

  storage.createChainStore(network);

  return storage;
};

module.exports = mockStorage;
