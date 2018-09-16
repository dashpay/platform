const { Wallet } = require('../index');

const mnemonic = 'never citizen worry shrimp used wild color snack undo armed scout chief';
const walletOpts = {
  mode: 'light',
  network: 'testnet',
  mnemonic,
};
const wallet = new Wallet(walletOpts);
const account = wallet.getAccount(0);

const startService = () => {
  // generate an unused address
  const unused = account.getUnusedAddress();

  // Import a tx that happened in the network
  // See for the format
  const addresses = {};
  account.storage.importAddresses(addresses, account.walletId);

  // Get any specific address
  const specific = account.getAddress(100);

  // Generate a batch of 200 addreses
  const poolAddresses = [];
  for (let i = 0; i < 200; i += 1) {
    poolAddresses.push(account.getAddress(i).address);
  }
};

account.events.on('ready', startService);
