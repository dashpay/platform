const Wallet = require('../src/types/Wallet/Wallet');

const wallet = new Wallet({
  network: 'testnet',
  privateKey: '9488797ee0018994d5ae22640e25210f65b1a450a0adcfa428cb5ffb29faa24b',
});

wallet
  .getAccount()
  .then(async (account) => {
    const address = account.getUnusedAddress();

    const balance = account.getTotalBalance();
    if (balance > 0) {
      const rawtx = account.createTransaction({
        recipient: address,
        satoshis: parseInt(balance / 4, 10),
      });
      await account.broadcastTransaction(rawtx);
    }
  });
