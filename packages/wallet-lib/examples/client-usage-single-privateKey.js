const Wallet = require('../src/types/Wallet/Wallet');

const privateKey = process.env.WALLET_PRIVATE_KEY;
if (!privateKey) {
  throw new Error('Set WALLET_PRIVATE_KEY to a testnet private key before running this example');
}

const wallet = new Wallet({
  network: 'testnet',
  privateKey,
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
