const { Message } = require('@dashevo/dashcore-lib');
const { Wallet } = require('../src/index');

const mnemonic = 'never citizen worry shrimp used wild color snack undo armed scout chief';
const walletOpts = {
  offlineMode: true,
  network: 'testnet',
  mnemonic,
};
const wallet = new Wallet(walletOpts);
wallet.getAccount()
  .then((account) => {
    const startSigningMessage = () => {
      const message = new Message('Hello world!');

      const idPrivateKey = account.identities.getIdentityHDKeyByIndex(0, 0).privateKey;

      const signed = account.sign(message, idPrivateKey);
      message.verify(idPrivateKey.toAddress().toString(), signed.toString()); // true
    };
    startSigningMessage();
  });
