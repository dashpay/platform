const { Wallet } = require('../src');

const wallet = new Wallet();

const account = wallet.getAccount(0);

account.events.on('ready', () => {
  console.log('Balance', account.getTotalBalance());
  console.log('Funding address', account.getUnusedAddress().address);
});
