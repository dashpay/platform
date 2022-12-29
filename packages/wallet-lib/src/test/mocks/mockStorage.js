const Storage = require('../../types/Storage/Storage');
const LocalForageAdapterMock = require('./LocalForageAdapterMock');

const defaultOptions = {
  withAdapter: false,
  network: 'testnet',
  autosaveIntervalTime: 500,
};

const mockStorage = async (opts = {}) => {
  const options = { ...defaultOptions, ...opts };
  const { network, withAdapter, autosaveIntervalTime } = options;

  const walletId = `walletId_${Math.random() * 100}`;

  const storage = new Storage();
  storage.currentWalletId = walletId;
  storage.currentNetwork = network;
  storage.autosaveIntervalTime = autosaveIntervalTime;

  await storage.configure({
    network,
    walletId,
    adapter: withAdapter ? new LocalForageAdapterMock() : null,
  });

  return storage;
};

module.exports = mockStorage;
