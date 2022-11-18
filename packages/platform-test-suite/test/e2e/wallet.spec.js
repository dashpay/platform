const Dash = require('dash');

const getDAPISeeds = require('../../lib/test/getDAPISeeds');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForBalanceToChange = require('../../lib/test/waitForBalanceToChange');

const { EVENTS } = Dash.WalletLib;

describe('e2e', () => {
  describe('Wallet', function main() {
    this.timeout(950000);

    let failed = false;
    let fundedWallet;
    let fundedAccount;
    let emptyWallet;
    let emptyWalletHeight;
    let emptyAccount;
    let restoredWallet;
    let restoredAccount;
    let mnemonic;
    let firstTransaction;
    let secondTransaction;

    before(async () => {
      fundedWallet = await createClientWithFundedWallet(10000);
      const network = process.env.NETWORK;
      emptyWallet = new Dash.Client({
        seeds: getDAPISeeds(),
        network,
        wallet: {
          waitForInstantLockTimeout: 120000,
        },
      });

      mnemonic = emptyWallet.wallet.exportWallet();
      const { storage } = fundedWallet.wallet;
      emptyWalletHeight = storage.getChainStore(storage.application.network).state.blockHeight;
    });

    // Skip test if any prior test in this describe failed
    beforeEach(function beforeEach() {
      if (failed) {
        this.skip();
      }
    });

    afterEach(function afterEach() {
      failed = this.currentTest.state === 'failed';
    });

    after(async () => {
      if (fundedWallet) {
        await fundedWallet.disconnect();
      }

      if (emptyWallet) {
        await emptyWallet.disconnect();
      }

      if (restoredWallet) {
        await restoredWallet.disconnect();
      }
    });

    describe('empty wallet', () => {
      it('should have no transaction at first', async () => {
        emptyAccount = await emptyWallet.getWalletAccount();

        expect(emptyAccount.getTransactions()).to.be.empty();
      });

      it('should receive a transaction when as it has been sent', async () => {
        fundedAccount = await fundedWallet.getWalletAccount();

        firstTransaction = await fundedAccount.createTransaction({
          recipient: emptyAccount.getUnusedAddress().address,
          satoshis: 1000,
        });

        await Promise.all([
          fundedAccount.broadcastTransaction(firstTransaction),
          waitForBalanceToChange(emptyAccount),
        ]);

        const transactionIds = Object.keys(emptyAccount.getTransactions());

        expect(transactionIds).to.have.lengthOf(1);

        expect(transactionIds[0]).to.equal(firstTransaction.id);
      });
    });

    describe('restored wallet', () => {
      it('should have all transaction from before at first', async () => {
        restoredWallet = new Dash.Client({
          wallet: {
            mnemonic,
            waitForInstantLockTimeout: 120000,
            unsafeOptions: {
              skipSynchronizationBeforeHeight: emptyWalletHeight,
            },
          },
          seeds: getDAPISeeds(),
          network: process.env.NETWORK,
        });

        restoredAccount = await restoredWallet.getWalletAccount();

        let transactions = restoredAccount.getTransactions();

        // Wait for new block if transaction has not been propagated yet
        if (Object.keys(transactions).length === 0) {
          await new Promise((resolve) => restoredAccount.once(EVENTS.BLOCKHEADER, resolve));
          transactions = restoredAccount.getTransactions();
        }

        await waitForBalanceToChange(restoredAccount);

        const transactionIds = Object.keys(transactions);

        expect(transactionIds).to.have.lengthOf(1);

        expect(transactionIds[0]).to.equal(firstTransaction.id);
      });

      it('should receive a transaction when as it has been sent', async () => {
        secondTransaction = await fundedAccount.createTransaction({
          recipient: restoredAccount.getUnusedAddress().address,
          satoshis: 1000,
        });

        await Promise.all([
          fundedAccount.broadcastTransaction(secondTransaction),
          waitForBalanceToChange(restoredAccount),
        ]);

        const transactionIds = Object.keys(restoredAccount.getTransactions());

        expect(transactionIds).to.have.lengthOf(2);

        expect(transactionIds).to.have.members([
          secondTransaction.id,
          firstTransaction.id,
        ]);
      });
    });

    describe('empty wallet', () => {
      it('should receive a transaction when as it has been sent to restored wallet', async () => {
        let transactionIds = Object.keys(emptyAccount.getTransactions());

        if (transactionIds.length < 2) {
          await waitForBalanceToChange(emptyAccount);
        }

        transactionIds = Object.keys(emptyAccount.getTransactions());

        expect(transactionIds).to.have.lengthOf(2);

        expect(transactionIds).to.have.members([
          firstTransaction.id,
          secondTransaction.id,
        ]);
      });
    });
  });
});
