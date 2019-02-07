const { Wallet, EVENTS } = require('../index');

const wallet = new Wallet({
  mnemonic: 'protect cave garden achieve hand vacant clarify atom finish outer waste sword',
  network: 'testnet',
});

const account = wallet.getAccount();
const start = async () => {
  console.log('Balance Conf', await account.getBalance(false, false));
  console.log('Balance Unconf', await account.getBalance(true, false));
  console.log('New Addr', await account.getUnusedAddress().address);
};
account.events.on(EVENTS.GENERATED_ADDRESS, (info) => { console.log('GENERATED_ADDRESS'); });
account.events.on(EVENTS.BALANCE_CHANGED, (info) => { console.log('Balance Changed', info, info.delta); });
account.events.on(EVENTS.UNCONFIRMED_BALANCE_CHANGED, (info) => { console.log('UNCONFIRMED_BALANCE_CHANGED', info); });
account.events.on(EVENTS.READY, start);
account.events.on(EVENTS.BLOCKHEIGHT_CHANGED, info => console.log('BLOCKHEIGHT_CHANGED:', info));
account.events.on(EVENTS.PREFETCHED, () => { console.log(EVENTS.PREFETCHED); });
account.events.on(EVENTS.DISCOVERY_STARTED, () => console.log(EVENTS.PREFETCHED));
