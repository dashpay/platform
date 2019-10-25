const DAPIClient = require('@dashevo/dapi-client');
const logger = require('../src/logger');
const { Wallet, EVENTS } = require('../src');

const transport = new DAPIClient({
  seeds: [{ service: '18.237.69.61:3000' }],
  timeout: 20000,
  retries: 5,
});
const wallet = new Wallet({
  mnemonic: 'protect cave garden achieve hand vacant clarify atom finish outer waste sword',
  network: 'testnet',
  transport,
});

const account = wallet.getAccount();
const start = async () => {
  logger.info('Balance Confirmed', await account.getConfirmedBalance(false));
  logger.info('Balance Unconfirmed', await account.getUnconfirmedBalance(false));
  logger.info('New Address', await account.getUnusedAddress().address);
};
account.events.on(EVENTS.GENERATED_ADDRESS, () => logger.info('GENERATED_ADDRESS'));
account.events.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => logger.info('CONFIRMED_BALANCE_CHANGED', info));
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => logger.info('UNCONFIRMED_BALANCE_CHANGED', info));
account.events.on(EVENTS.READY, start);
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => logger.info('BLOCKHEIGHT_CHANGED:', info));
account.events.on(EVENTS.PREFETCHED, () => logger.info('PREFETCHED', EVENTS.PREFETCHED));
account.events.on(EVENTS.DISCOVERY_STARTED, () => logger.info(EVENTS.PREFETCHED));
