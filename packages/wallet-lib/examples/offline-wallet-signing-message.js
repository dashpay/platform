const { Message } = require('@dashevo/dashcore-lib');
const { Wallet } = require('../src/index');

const mnemonic = 'never citizen worry shrimp used wild color snack undo armed scout chief';
const walletOpts = {
  offlineMode: true,
  network: 'testnet',
  mnemonic,
};
const wallet = new Wallet(walletOpts);
const account = wallet.getAccount();

const startSigningMessage = () => {
  const message = new Message('Hello world!');

  const idPrivateKey = account.getIdentityHDKey().privateKey;

  const signed = account.sign(message, idPrivateKey);
  const verify = message.verify(idPrivateKey.toAddress().toString(), signed.toString()); // true
};

account.on('ready', startSigningMessage);
