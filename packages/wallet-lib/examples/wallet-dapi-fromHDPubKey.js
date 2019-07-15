const DAPIClient = require('@dashevo/dapi-client');
const { Wallet, EVENTS } = require('../src');

const transport = new DAPIClient({
  seeds: [{ service: '18.237.69.61:3000' }],
  timeout: 20000,
  retries: 5,
});
const wallet = new Wallet({
  HDExtPublicKey: 'tpubDFLd6pf4VJ72YFdAVp5sXWiszJhGM4yhXnAYkXagFf2fS3NiTU6rwQsxkMiVPKqeGBTWC2DZ8ZicuT49jnKwMEr6gAT4f83YqB3dnujarD3',
  network: 'testnet',
  transport,
});


const account = wallet.getAccount();


const start = async () => {
  console.log('Balance Conf', await account.getConfirmedBalance(false));
  console.log('Balance Unconf', await account.getUnconfirmedBalance( false));
  console.log('New Addr', await account.getUnusedAddress().address);
  //
  // const tx = account.createTransaction({recipient:'yhvXpqQjfN9S4j5mBKbxeGxiETJrrLETg5', amount:5.74});
  // console.log(tx.toString());
  // const bdc = await account.broadcastTransaction(tx.toString());
  // console.log(bdc)
};

account.events.on(EVENTS.GENERATED_ADDRESS, (info) => { console.log('GENERATED_ADDRESS'); });
account.events.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => { console.log('CONFIRMED_BALANCE_CHANGED', info, info.delta); });
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => { console.log('UNCONFIRMED_BALANCE_CHANGED', info); });
account.events.on(EVENTS.READY, start);
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, info => console.log('BLOCKHEIGHT_CHANGED:', info));
account.events.on(EVENTS.PREFETCHED, () => { console.log(EVENTS.PREFETCHED); });
account.events.on(EVENTS.DISCOVERY_STARTED, () => console.log(EVENTS.PREFETCHED));
