const { NodeForage } = require('nodeforage');
const { Wallet } = require('@dashevo/wallet-lib');
const { PrivateKey } = require('@dashevo/dashcore-lib');
const fundWallet = require('@dashevo/wallet-lib/src/utils/fundWallet');
const getDAPISeeds = require('../../lib/test/getDAPISeeds');
const createFaucetClient = require('../../lib/test/createFaucetClient');

const forage = new NodeForage({ name: 'users' });

describe('Storage', () => {
  const createWallet = () => {
    const wallet = new Wallet({
      privateKey: process.env.FAUCET_PRIVATE_KEY,
      transport: {
        seeds: getDAPISeeds(),
      },
      network: process.env.NETWORK,
      adapter: forage,
      mnemonic: 'fantasy blood fire buzz glimpse wrap mule right sponsor define hospital lonely',
    });
    return wallet;
  };

  it.only('should initialize faucet wallet and fill storage', async () => {
    const wallet = createWallet();
    const account = await wallet.getAccount();
    console.log(account.getTransactionHistory());
    await new Promise((resolve) => setTimeout(resolve, 20000));
  });

  it('should fill with faucet', async () => {
    const wallet = createWallet();
    const faucetClient = createFaucetClient();

    await fundWallet(faucetClient.wallet, wallet, 1000);
  });

  it('should load faucet wallet from storage and send transactions', async () => {
    const wallet = createWallet();
    const account = await wallet.getAccount();
    for (let i = 0; i < 1; i++) {
      const tx = account.createTransaction({
        recipient: new PrivateKey().toAddress().toString(),
        satoshis: 1000,
      });
      await account.broadcastTransaction(tx);
      console.log('Broadcasted', tx.hash, i);
    }
  });
});
