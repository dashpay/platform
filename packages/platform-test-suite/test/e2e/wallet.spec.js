const Dash = require('dash');

const getDAPISeeds = require('../../lib/test/getDAPISeeds');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForBalanceToChange = require('../../lib/test/waitForBalanceToChange');
const wait = require('../../lib/wait');

const TRANSACTION_PROPAGATION_TIMEOUT_MS = 120000;
const TRANSACTION_POLL_INTERVAL_MS = 500;

async function waitForTransaction(account, transactionId) {
  const deadline = Date.now() + TRANSACTION_PROPAGATION_TIMEOUT_MS;
  let transactions = account.getTransactions();

  while (!transactions[transactionId] && Date.now() < deadline) {
    await wait(TRANSACTION_POLL_INTERVAL_MS);
    transactions = account.getTransactions();
  }

  if (!transactions[transactionId]) {
    throw new Error(
      `Transaction ${transactionId} did not reach the account within ${TRANSACTION_PROPAGATION_TIMEOUT_MS}ms`,
    );
  }

  return transactions;
}

describe('e2e', function e2eTest() {
  this.bail(true);

  describe('Wallet', function main() {
    this.timeout(950000);

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

    before(async function createClients() {
      // TODO: temporarily disabled on browser because of header stream is not syncing
      //   headers at some point. Our theory is that because wallets aren't offloading properly
      //   and we have too many streams open.
      if (typeof window !== 'undefined') {
        this.skip('temporarily disabled on browser because of header stream is not syncing'
          + ' headers at some point. Our theory is that because wallets aren\'t offloading'
          + ' properly and we have too many streams open.');
      }

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
      emptyWalletHeight = storage.getChainStore(storage.application.network).state.chainHeight;
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

        // A mempool transaction may need the next block before a restored wallet can discover it.
        const transactions = await waitForTransaction(restoredAccount, firstTransaction.id);

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

        const transactions = await waitForTransaction(restoredAccount, secondTransaction.id);
        const transactionIds = Object.keys(transactions);

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

        const transactions = await waitForTransaction(emptyAccount, secondTransaction.id);
        transactionIds = Object.keys(transactions);

        expect(transactionIds).to.have.lengthOf(2);

        expect(transactionIds).to.have.members([
          firstTransaction.id,
          secondTransaction.id,
        ]);
      });
    });
  });
});
