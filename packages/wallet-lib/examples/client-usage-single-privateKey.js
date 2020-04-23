const Wallet = require('../src/types/Wallet/Wallet');

const wallet = new Wallet({
  network: 'testnet',
  privateKey: '9488797ee0018994d5ae22640e25210f65b1a450a0adcfa428cb5ffb29faa24b',
});
const account = wallet.getAccount();

const start = async () => {
  console.log('PrivateKey:', wallet.exportWallet());

  const address = account.getUnusedAddress();
  console.log('Unused Address :', address);

  const balance = account.getTotalBalance();
  console.log('balance :', balance);

  const rawtx = account.createTransaction({
    recipient: address,
    satoshis: parseInt(balance / 4, 10),
  });
  console.log(rawtx);

  const txid = await account.broadcastTransaction(rawtx);
  console.log('txid:', txid);
};
account.on('ready', start);
