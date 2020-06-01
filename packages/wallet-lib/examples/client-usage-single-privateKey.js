const Wallet = require('../src/types/Wallet/Wallet');

const wallet = new Wallet({
  network: 'testnet',
  privateKey: '9488797ee0018994d5ae22640e25210f65b1a450a0adcfa428cb5ffb29faa24b',
});

console.log('PrivateKey:', wallet.exportWallet());

wallet
  .getAccount()
  .then(async (account) => {
    const address = account.getUnusedAddress();
    console.log('Unused Address :', address);

    const balance = account.getTotalBalance();
    console.log('balance :', balance);
    if (balance > 0) {
      const rawtx = account.createTransaction({
        recipient: address,
        satoshis: parseInt(balance / 4, 10),
      });
      const txid = await account.broadcastTransaction(rawtx);
      console.log('txid:', txid);
    }
  }).catch((e) => {
    console.log('Failed with error', e);
  });
