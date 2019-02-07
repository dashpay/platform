const { Wallet } = require('../index');

const wallet = new Wallet();

const account = wallet.getAccount(0);

const start = () => {
  console.log('Balance', account.getBalance());
  console.log('Funding address', account.getUnusedAddress().address);
};

account.events.on('ready', start);
