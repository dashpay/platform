const { Wallet, EVENTS } = require('../src');
const logger = require('../src/logger');

const wallet = new Wallet({
  mnemonic: 'smart rug aspect stuff auction bridge title virtual illegal enact black since', // Werner - dev (10 Nov)
  network: 'testnet',
  transport: 'insight',
});


const account = wallet.getAccount();
const start = async () => {
  logger.info('Balance Confirmed', await account.getConfirmedBalance(false));
  logger.info('Balance Unconfirmed', await account.getUnconfirmedBalance(false));
  logger.info('New Address', await account.getUnusedAddress());
};
account.events.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => logger.info('CONFIRMED_BALANCE_CHANGED', info));
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => logger.info('UNCONFIRMED_BALANCE_CHANGED', info));
account.events.on(EVENTS.READY, start);
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => logger.info('BLOCKHEIGHT_CHANGED', info));
account.events.on(EVENTS.PREFETCHED, () => logger.info('PREFETCHED', EVENTS.PREFETCHED));
account.events.on(EVENTS.DISCOVERY_STARTED, () => logger.info('DISCOVERY_STARTED', EVENTS.PREFETCHED));
