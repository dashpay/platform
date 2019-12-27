const { Wallet } = require('../src/index');
const logger = require('../src/logger');

const mnemonic = 'never citizen worry shrimp used wild color snack undo armed scout chief';
const walletOpts = {
  offlineMode: true,
  network: 'testnet',
  mnemonic,
  // HDPublicKey:
};
const wallet = new Wallet(walletOpts);
const account = wallet.getAccount(0);

/**
 * This simple offline service simulates an offline usage for generating new addresses from an Extended Pub Key.
 */
const startService = () => {
  // Import a tx that happened in the network
  // See for the format
  const addresses = {};
  account.storage.importAddresses(addresses, account.walletId);

  // Get any specific address
  const specific = account.getAddress(100);
  logger.info('Specific', specific);

  // Generate a batch of all 200 first addreses
  const poolAddresses = [];
  for (let i = 0; i <= 200; i += 1) {
    poolAddresses.push(account.getAddress(i).address);
  }
  logger.info('Pregenerated pool of addr', poolAddresses);

  const addrPool = [];
  // get 10 unused address
  for (let i = 0; i < 10; i += 1) {
    const skip = i;
    addrPool.push(account.getUnusedAddress('external', skip));
  }
  logger.info('Pool of unused addr', addrPool);
};

account.events.on('ready', startService);
