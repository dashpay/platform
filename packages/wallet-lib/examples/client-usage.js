const logger = require('../src/logger');
const { Wallet, EVENTS } = require('../src');

const wallet = new Wallet({
  mnemonic: 'protect cave garden achieve hand vacant clarify atom finish outer waste sword',
  network: 'testnet',
});

const account = wallet.getAccount();
const start = async () => {
  logger.info('Balance Confirmed', await account.getConfirmedBalance(false));
  logger.info('Balance Unconfirmed', await account.getUnconfirmedBalance(false));
  logger.info('New Address', await account.getUnusedAddress().address);

  const transaction = account.createTransaction({ satoshis: 1000, recipient: 'ycyFFyWCPSWbXLZBeYppJqgvBF7bnu8BWQ' });
  const transactionID = await account.broadcastTransaction(transaction);
};
account.on(EVENTS.GENERATED_ADDRESS, () => logger.info('GENERATED_ADDRESS'));
account.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => logger.info('CONFIRMED_BALANCE_CHANGED', info));
account.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => logger.info('UNCONFIRMED_BALANCE_CHANGED', info));
account.on(EVENTS.READY, start);
account.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => logger.info('BLOCKHEIGHT_CHANGED:', info));
account.on(EVENTS.PREFETCHED, () => logger.info('PREFETCHED', EVENTS.PREFETCHED));
account.on(EVENTS.DISCOVERY_STARTED, () => logger.info(EVENTS.PREFETCHED));
