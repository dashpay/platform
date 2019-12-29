const DAPIClient = require('@dashevo/dapi-client');
const logger = require('../src/logger');
const { Wallet, EVENTS } = require('../src');

const transport = new DAPIClient({
  seeds: [{ service: '18.236.131.253:3000' }],
  timeout: 20000,
  retries: 5,
});
const wallet = new Wallet({
  HDPublicKey: 'tpubDFLd6pf4VJ72YFdAVp5sXWiszJhGM4yhXnAYkXagFf2fS3NiTU6rwQsxkMiVPKqeGBTWC2DZ8ZicuT49jnKwMEr6gAT4f83YqB3dnujarD3',
  network: 'testnet',
  transport,
});


const account = wallet.getAccount();


const start = async () => {
  logger.info('Balance Confirmed', await account.getConfirmedBalance(false));
  logger.info('Balance Unconfirmed', await account.getUnconfirmedBalance(false));
  logger.info('New Address', await account.getUnusedAddress().address);
  //
  // const tx = account.createTransaction({recipient:'yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5', amount:5.74});
  // console.log(tx.toString());
  // const bdc = await account.broadcastTransaction(tx.toString());
  // console.log(bdc)
};

account.events.on(EVENTS.GENERATED_ADDRESS, () => logger.info('GENERATED_ADDRESS'));
account.events.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => logger.info('CONFIRMED_BALANCE_CHANGED', info));
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => logger.info('UNCONFIRMED_BALANCE_CHANGED', info));
account.events.on(EVENTS.READY, start);
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, (info) => logger.info('BLOCKHEIGHT_CHANGED:', info));
account.events.on(EVENTS.PREFETCHED, () => logger.info('EVENTS_PREFETCHED', EVENTS.PREFETCHED));
account.events.on(EVENTS.DISCOVERY_STARTED, () => logger.info('EVENTS_DISCOVERY_STARTED', EVENTS.PREFETCHED));
