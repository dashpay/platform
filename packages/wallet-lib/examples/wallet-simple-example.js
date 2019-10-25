const { Wallet } = require('../src');
const logger = require('../src/logger');

const wallet = new Wallet();

const account = wallet.getAccount(0);

account.events.on('ready', () => {
  logger.info('Balance', account.getTotalBalance());
  logger.info('Funding address', account.getUnusedAddress().address);
});
