const Wallet = require('../src/Wallet/Wallet');

const transport = 'insight';
const privateKey = '9488797ee0018994d5ae22640e25210f65b1a450a0adcfa428cb5ffb29faa24b';
const network = 'testnet';
const start = async () => {
  const wallet = new Wallet({
    network,
    transport,
    privateKey,
  });
  console.log('Private:', wallet.exportWallet());
  console.log('Type:', wallet.type);
  const account = wallet.getAccount(0);

  const address = account.getAddress();
  console.log('Address :', address);


  account.events.on('ready', async () => {
    const balance = account.getBalance();
    console.log('balance :', balance);

    const rawtx = account.createTransaction({
      recipient: address,
      satoshis: balance - 10000,
      isInstantSend: true,
    });
    console.log(rawtx);

    const txid = await account.broadcastTransaction(rawtx);
    console.log('txid:', txid);
  });
};
start();
