const logger = require('../src/logger');
const { Wallet, EVENTS } = require('../src');

const wallet = new Wallet({
  HDPublicKey: 'tpubDFLd6pf4VJ72YFdAVp5sXWiszJhGM4yhXnAYkXagFf2fS3NiTU6rwQsxkMiVPKqeGBTWC2DZ8ZicuT49jnKwMEr6gAT4f83YqB3dnujarD3',
  network: 'testnet',
});


wallet.getAccount()
  .then(async (account) => {
    logger.info('Balance Confirmed', await account.getConfirmedBalance(false));
    logger.info('Balance Unconfirmed', await account.getUnconfirmedBalance(false));
    logger.info('New Address', await account.getUnusedAddress().address);

    account.on(EVENTS.GENERATED_ADDRESS, () => logger.info('GENERATED_ADDRESS'));
    account.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => logger.info('CONFIRMED_BALANCE_CHANGED', info));
    account.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => logger.info('UNCONFIRMED_BALANCE_CHANGED', info));
    account.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => logger.info('BLOCKHEIGHT_CHANGED:', info));
    account.on(EVENTS.PREFETCHED, () => logger.info('EVENTS_PREFETCHED', EVENTS.PREFETCHED));
    account.on(EVENTS.DISCOVERY_STARTED, () => logger.info('EVENTS_DISCOVERY_STARTED', EVENTS.PREFETCHED));
  });
