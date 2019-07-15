const { Wallet, EVENTS } = require('../index');

const wallet = new Wallet({
  mnemonic: 'smart rug aspect stuff auction bridge title virtual illegal enact black since', // Werner - dev (10 Nov)
  network: 'testnet',
  transport: 'insight',
});


const account = wallet.getAccount();
const start = async () => {
  console.log('Balance Conf', await account.getConfirmedBalance( false));
  console.log('Balance Unconf', await account.getUnconfirmedBalance( false));
  console.log('New Addr', await account.getUnusedAddress());
};
account.events.on(EVENTS.CONFIRMED_BALANCE_CHANGED, (info) => { console.log('CONFIRMED_BALANCE_CHANGED', info, info.delta); });
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => { console.log('UNCONFIRMED_BALANCE_CHANGED', info); });
account.events.on(EVENTS.READY, start);
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, info => console.log(info));
account.events.on(EVENTS.PREFETCHED, () => { console.log(EVENTS.PREFETCHED); });
account.events.on(EVENTS.DISCOVERY_STARTED, () => console.log(EVENTS.PREFETCHED));
