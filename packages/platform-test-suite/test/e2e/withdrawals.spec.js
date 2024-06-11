const { expect } = require('chai');

const wait = require('@dashevo/dapi-client/lib/utils/wait');
const { STATUSES: WITHDRAWAL_STATUSES } = require('dash/build/SDK/Client/Platform/methods/identities/creditWithdrawal');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../lib/waitForSTPropagated');

describe('Withdrawals', function withdrawalsTest() {
  this.bail(true);

  let client;
  let identity;

  before(async function createClients() {
    // TODO: temporarily disabled on browser because of header stream is not syncing
    //   headers at some point. Our theory is that because wallets aren't offloading properly
    //   and we have too many streams open.
    if (typeof window !== 'undefined') {
      this.skip('temporarily disabled on browser because of header stream is not syncing'
        + ' headers at some point. Our theory is that because wallets aren\'t offloading properly'
        + ' and we have too many streams open.');
    }

    client = await createClientWithFundedWallet(
      10000000,
    );

    await client.platform.initialize();
  });

  after(async () => {
    if (client) {
      await client.disconnect();
    }
  });

  describe('Any Identity', () => {
    const INITIAL_BALANCE = 1000000;

    before(async () => {
      identity = await client.platform.identities.register(INITIAL_BALANCE);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();
    });

    it('should be able to withdraw credits', async () => {
      const account = await client.getWalletAccount();
      const walletBalanceBefore = account.getTotalBalance();
      const identityBalanceBefore = identity.getBalance();
      const withdrawTo = await account.getUnusedAddress();
      const amountToWithdraw = 1000000;

      await client.platform.identities.withdrawCredits(
        identity,
        BigInt(amountToWithdraw),
        withdrawTo.address,
      );

      // Re-fetch identity to obtain latest core chain lock height
      identity = await client.platform.identities.get(identity.getId());
      const identityMetadata = identity.getMetadata().toObject();
      const { coreChainLockedHeight: initialCoreChainLockedHeight } = identityMetadata;

      // Wait for core chain lock update.
      // After that drive should update document status to completed.
      // (Wait 2 chainlocks on regtest since they are processed quicker,
      // and withdrawal might not complete yet)
      const chainLocksToWait = process.env.NETWORK === 'regtest' ? 2 : 1;
      const { promise } = await client.platform.identities.utils
        .waitForCoreChainLockedHeight(initialCoreChainLockedHeight + chainLocksToWait);
      await promise;

      // Wait for document completion to propagate
      await waitForSTPropagated();

      // Wait for document status to be changed to COMPLETED.
      let withdrawalCompleted = false;
      let withdrawalDocument;
      for (let i = 0; i < 10; i++) {
        const withdrawals = await client.platform
          .documents.get(
            'withdrawals.withdrawal',
            {
              where: [['$ownerId', '==', identity.getId()]],
            },
          );

        withdrawalDocument = withdrawals[withdrawals.length - 1];
        withdrawalCompleted = withdrawalDocument.get('status') === WITHDRAWAL_STATUSES.COMPLETED;

        if (withdrawalCompleted) {
          break;
        }

        await waitForSTPropagated();
      }

      expect(withdrawalCompleted).to.be.true();

      const walletBalanceUpdated = account.getTotalBalance();

      identity = await client.platform.identities.get(identity.getId());
      const identityBalanceUpdated = identity.getBalance();

      // Should ensure balances are right
      expect(walletBalanceUpdated).to.be.greaterThan(walletBalanceBefore);
      expect(identityBalanceUpdated).to.be.lessThan(identityBalanceBefore);

      // Should allow deleting of the withdrawal document
      await client.platform.documents.broadcast({
        delete: [withdrawalDocument],
      }, identity);
    });

    it('should be able to query recent withdrawal updates', async () => {
      const account = await client.getWalletAccount();
      const withdrawTo = await account.getUnusedAddress();
      const amountToWithdraw = 1000000;

      const firstWithdrawalTime = Date.now();
      const { height: withdrawalHeight } = await client.platform.identities.withdrawCredits(
        identity,
        BigInt(amountToWithdraw),
        withdrawTo.address,
      );

      let withdrawalBroadcasted = false;
      let blocksPassed = 0;

      // Wait for first withdrawal to broadcast
      while (!withdrawalBroadcasted && blocksPassed === 0) {
        await waitForSTPropagated();

        const withdrawals = await client.platform
          .documents.get(
            'withdrawals.withdrawal',
            {
              where: [
                ['$ownerId', '==', identity.getId()],
                ['$updatedAt', '>', firstWithdrawalTime],
              ],
              orderBy: [
                ['$updatedAt', 'desc'],
              ],
            },
          );

        // We want to ensure that our index works properly with updatedAt
        // condition, and we are not receiving the document from previous test
        expect(withdrawals.length).to.equal(1);

        const withdrawal = withdrawals[0];

        withdrawalBroadcasted = withdrawal.get('status') === WITHDRAWAL_STATUSES.BROADCASTED;

        blocksPassed = withdrawal.getMetadata()
          .getBlockHeight() - withdrawalHeight;
      }

      expect(withdrawalBroadcasted).to.be.true();
    });

    it('should not be able to withdraw more than balance available', async () => {
      const account = await client.getWalletAccount();
      const identityBalanceBefore = identity.getBalance();
      const withdrawTo = await account.getUnusedAddress();
      const amountToWithdraw = identityBalanceBefore * 2;

      await expect(client.platform.identities.withdrawCredits(
        identity,
        BigInt(amountToWithdraw),
        withdrawTo.address,
      )).to.be.rejectedWith(`Withdrawal amount "${amountToWithdraw}" is bigger that identity balance "${identityBalanceBefore}"`);
    });

    it('should not allow to create withdrawal with wrong security key type', async () => {
      const account = await client.getWalletAccount();
      const identityBalanceBefore = identity.getBalance();
      const withdrawTo = await account.getUnusedAddress();
      const amountToWithdraw = identityBalanceBefore / 2;

      await expect(client.platform.identities.withdrawCredits(
        identity,
        BigInt(amountToWithdraw),
        withdrawTo.address,
        {
          signingKeyIndex: 1,
        },
      )).to.be.rejectedWith('Error conversion not implemented: Invalid public key security level HIGH. The state transition requires one of CRITICAL');
    });

    it('should not be able to create withdrawal document', async () => {
      const withdrawal = await client.platform.documents.create(
        'withdrawals.withdrawal',
        identity,
        {
          amount: 1000,
          coreFeePerByte: 1365,
          pooling: 0,
          outputScript: Buffer.alloc(23),
          status: 0,
        },
      );

      try {
        await client.platform.documents.broadcast({
          create: [withdrawal],
        }, identity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal('Action is not allowed');
        expect(e.code).to.equal(40500);
      }
    });

    it('should not be able to delete incomplete withdrawal document', async () => {
      const account = await client.getWalletAccount();
      const withdrawTo = await account.getUnusedAddress();

      await client.platform.identities.withdrawCredits(
        identity,
        BigInt(1000000),
        withdrawTo.address,
      );

      await waitForSTPropagated();

      let withdrawalBroadcasted = false;
      let withdrawalDocument;
      // Wait for withdrawal to broadcast, otherwise there's a chance
      // that test will try update document at the same time with the drive itself
      while (!withdrawalBroadcasted) {
        ([withdrawalDocument] = await client.platform
          .documents.get(
            'withdrawals.withdrawal',
            {
              where: [['$ownerId', '==', identity.getId()]],
            },
          ));

        withdrawalBroadcasted = withdrawalDocument.get('status') === WITHDRAWAL_STATUSES.BROADCASTED;

        await wait(1000);
      }

      try {
        await client.platform.documents.broadcast({
          delete: [withdrawalDocument],
        }, identity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal('withdrawal deletion is allowed only for COMPLETE statuses');
        expect(e.code).to.equal(40500);
      }
    });

    it('should not be able to update withdrawal document', async () => {
      const [withdrawalDocument] = await client.platform
        .documents.get(
          'withdrawals.withdrawal',
          {
            where: [['$ownerId', '==', identity.getId()]],
          },
        );

      withdrawalDocument.set('status', 3);

      try {
        await client.platform.documents.broadcast({
          replace: [withdrawalDocument],
        }, identity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal('Action is not allowed');
        expect(e.code).to.equal(40500);
      }
    });
  });
});
