const { expect } = require('chai');

const { Wallet } = require('../../src/index');

const { fundWallet } = require('../../src/utils');
const { EVENTS } = require('../../src');

const seeds = process.env.DAPI_SEED
  .split(',');

let newWallet;
let wallet;
let account;
let faucetWallet;

describe('Wallet-lib - functional ', function suite() {
  this.timeout(700000);

  before(() => {
    faucetWallet = new Wallet({
      transport: {
        seeds,
      },
      network: process.env.NETWORK,
      privateKey: process.env.FAUCET_PRIVATE_KEY
    });
  });

  after('Disconnection', () => {
    account.disconnect();
    wallet.disconnect();
    newWallet.disconnect();
    faucetWallet.disconnect();
  });

  describe('Wallet', () => {
    describe('Create a new Wallet', () => {
      it('should create a new wallet with default params', () => {
        newWallet = new Wallet({
          transport: {
            seeds,
          },
          network: process.env.NETWORK,
        });

        expect(newWallet.walletType).to.be.equal('hdwallet');
        expect(newWallet.plugins).to.be.deep.equal({});
        expect(newWallet.accounts).to.be.deep.equal([]);
        expect(newWallet.keyChain.type).to.be.deep.equal('HDPrivateKey');
        expect(newWallet.passphrase).to.be.deep.equal(null);
        expect(newWallet.allowSensitiveOperations).to.be.deep.equal(false);
        expect(newWallet.injectDefaultPlugins).to.be.deep.equal(true);
        expect(newWallet.walletId).to.length(10);
        expect(newWallet.network).to.be.deep.equal('testnet');

        const exported = newWallet.exportWallet();
        expect(exported.split(' ').length).to.equal(12);
      });
    });

    describe('Load a wallet', () => {
      it('should load a wallet from mnemonic', () => {
        wallet = new Wallet({
          mnemonic: newWallet.mnemonic,
          transport: {
            seeds,
          },
          network: process.env.NETWORK,
        });

        expect(wallet.walletType).to.be.equal('hdwallet');
        expect(wallet.plugins).to.be.deep.equal({});
        expect(wallet.accounts).to.be.deep.equal([]);
        expect(wallet.keyChain.type).to.be.deep.equal('HDPrivateKey');
        expect(wallet.passphrase).to.be.deep.equal(null);
        expect(wallet.allowSensitiveOperations).to.be.deep.equal(false);
        expect(wallet.injectDefaultPlugins).to.be.deep.equal(true);
        expect(wallet.walletId).to.length(10);
        expect(wallet.network).to.be.deep.equal('testnet');

        const exported = wallet.exportWallet();
        expect(exported).to.equal(newWallet.mnemonic);
      });
    });
  });

  describe('Account', () => {
    it('should await readiness', async () => {
      account = await wallet.getAccount();
      await account.isReady();
      expect(account.state.isReady).to.be.deep.equal(true);
    });

    it('populate balance with dash', async () => {
      const balanceBeforeTopUp = account.getTotalBalance();
      const amountToTopUp = 20000;

      await fundWallet(
        faucetWallet,
        wallet,
        amountToTopUp
      );

      const balanceAfterTopUp = account.getTotalBalance();
      const transactions = account.getTransactions();

      expect(Object.keys(transactions).length).to.be.equal(1);
      expect(balanceBeforeTopUp).to.be.equal(0);
      expect(balanceAfterTopUp).to.be.equal(amountToTopUp);
    });

    it('should has unusedAddress with index 1', () => {
      const unusedAddress = account.getUnusedAddress();
      expect(unusedAddress.index).to.equal(1);
    });

    it('should not have empty balance', () => {
      expect(account.getTotalBalance()).to.not.equal(0);
    });

    it('should returns some available UTXO', () => {
      const UTXOs = account.getUTXOS();
      expect(UTXOs.length).to.not.equal(0);
    });

    it('should create a transaction', () => {
      const newTx = account.createTransaction({
        recipient: 'ydvgJ2eVSmdKt78ZSVBJ7zarVVtdHGj3yR',
        satoshis: Math.floor(account.getTotalBalance() / 2)
      });

      expect(newTx.constructor.name).to.equal('Transaction');
      expect(newTx.outputs.length).to.not.equal(0);
      expect(newTx.inputs.length).to.not.equal(0);
    });

    it('should be able to restore wallet to the same state with a mnemonic', async () => {
      const restoredWallet = new Wallet({
        mnemonic: wallet.mnemonic,
        transport: {
          seeds,
        },
        network: wallet.network,
      });
      const restoredAccount = await restoredWallet.getAccount();

      // Due to the limitations of DAPI, we need to wait for a block to be mined if we connected in the
      // moment when transaction already entered the mempool, but haven't been mined yet
      await new Promise(resolve => restoredAccount.once(EVENTS.BLOCKHEADER, resolve));

      const expectedAddresses = account.getAddresses();
      const expectedTransactions = account.getTransactions();

      const addresses = restoredAccount.getAddresses();
      const transactions = restoredAccount.getTransactions();

      expect(Object.keys(transactions).length).to.be.equal(1);
      expect(addresses).to.be.deep.equal(expectedAddresses);
      expect(Object.keys(transactions)).to.be.deep.equal(Object.keys(expectedTransactions));
    });
  });
});
