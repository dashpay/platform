const Wallet = require('../src/Wallet');
const InsightClient = require('../src/transports/Insight/insightClient');

const wallet = new Wallet({
  mnemonic: 'wisdom claim quote stadium input danger planet angry crucial cargo struggle medal',
  network: 'testnet',
  transport: new InsightClient(),
});

const account = wallet.createAccount();

account.events.on('prefetched', () => {
  console.log('prefetched');
});
account.events.on('discovery_started', () => {
  console.log('discovery_started');
});
account.events.on('ready', async () => {
  // console.log(account.addresses)
  console.log('Funding address', account.getAddress(0, true).address);
  console.log('---');
  console.log('Balance', account.getBalance());
  const { address } = account.getUnusedAddress(true);
  console.log('Send to a child address', address);
  const isIs = true;
  const amount = parseInt(account.getBalance() / 2, 10);
  const rawTx = account.createTransaction({
    to: address,
    amount,
    isInstantSend: isIs,
  });
  console.log('Will pay', amount, 'in is to', address);
  console.log('Created rawtx', rawTx);
  const txid = await account.broadcastTransaction(rawTx, true);
  console.log('Broadcasted:', txid);
});
