const DashPlatformProtocol = require('@dashevo/dpp');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const { createFakeInstantLock } = require('dash/build/src/utils/createFakeIntantLock');
const { default: createAssetLockTransaction } = require('dash/build/src/SDK/Client/Platform/createAssetLockTransaction');

const waitForBlocks = require('../../../lib/waitForBlocks');
const waitForBalanceToChange = require('../../../lib/test/waitForBalanceToChange');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Platform', () => {
  describe('Identity', () => {
    let dpp;
    let client;
    let identity;
    let walletAccount;
    let walletPublicKey;

    before(async () => {
      dpp = new DashPlatformProtocol();

      client = await createClientWithFundedWallet();
      walletAccount = await client.getWalletAccount();
      ({
        publicKey: walletPublicKey,
      } = walletAccount.getIdentityHDKeyByIndex(0, 0));
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it.skip('should fail to create an identity if instantLock is not valid', async () => {
      const {
        transaction,
        privateKey,
        outputIndex,
      } = await createAssetLockTransaction({
        client,
      }, 1);

      await client.getDAPIClient().core.broadcastTransaction(transaction.toBuffer());
      await waitForBlocks(client.getDAPIClient(), 1);

      const invalidInstantLock = createFakeInstantLock(transaction.hash);
      const assetLockProof = await dpp.identity.createInstantAssetLockProof(invalidInstantLock);

      const invalidIdentity = dpp.identity.create(
        transaction,
        outputIndex,
        assetLockProof,
        [walletPublicKey],
      );

      const invalidIdentityCreateTransition = dpp.identity.createIdentityCreateTransition(
        invalidIdentity,
      );

      invalidIdentityCreateTransition.signByPrivateKey(privateKey);

      try {
        await client.getDAPIClient().platform.broadcastStateTransition(
          invalidIdentityCreateTransition.toBuffer(),
        );
        expect.fail('Error was not thrown');
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors'));
        expect(error.name).to.equal('IdentityAssetLockTransactionNotFoundError');
      }
    });

    it('should create an identity', async () => {
      identity = await client.platform.identities.register(3);

      expect(identity).to.exist();

      await waitForBalanceToChange(walletAccount);
    });

    it('should fail to create an identity with already used asset lock output');

    it('should fail to create an identity with already used public key', async () => {
      const {
        transaction,
        privateKey,
        outputIndex,
      } = await createAssetLockTransaction({ client }, 1);

      await client.getDAPIClient().core.broadcastTransaction(transaction.toBuffer());
      await waitForBlocks(client.getDAPIClient(), 1);

      const instantLock = createFakeInstantLock(transaction.hash);
      const assetLockProof = await dpp.identity.createInstantAssetLockProof(instantLock);

      const duplicateIdentity = dpp.identity.create(
        transaction,
        outputIndex,
        assetLockProof,
        [walletPublicKey],
      );

      const duplicateIdentityCreateTransition = dpp.identity.createIdentityCreateTransition(
        duplicateIdentity,
      );

      duplicateIdentityCreateTransition.signByPrivateKey(
        privateKey,
      );

      try {
        await client.getDAPIClient().platform.broadcastStateTransition(
          duplicateIdentityCreateTransition.toBuffer(),
        );

        expect.fail('Error was not thrown');
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors'));
        expect(error.name).to.equal('IdentityPublicKeyAlreadyExistsError');
        expect(Buffer.from(error.publicKeyHash)).to.deep.equal(identity.getPublicKeyById(0).hash());
      }
    });

    it('should be able to get newly created identity', async () => {
      const fetchedIdentity = await client.platform.identities.get(
        identity.getId(),
      );

      expect(fetchedIdentity).to.be.not.null();

      const fetchedIdentityWithoutBalance = fetchedIdentity.toJSON();
      delete fetchedIdentityWithoutBalance.balance;

      const localIdentityWithoutBalance = identity.toJSON();
      delete localIdentityWithoutBalance.balance;

      expect(fetchedIdentityWithoutBalance).to.deep.equal(localIdentityWithoutBalance);

      expect(fetchedIdentity.getBalance()).to.be.greaterThan(0);
    });

    it('should be able to get newly created identity by it\'s first public key', async () => {
      const [serializedIdentity] = await client.getDAPIClient().platform
        .getIdentitiesByPublicKeyHashes(
          [identity.getPublicKeyById(0).hash()],
        );

      expect(serializedIdentity).to.be.not.null();

      const receivedIdentity = dpp.identity.createFromBuffer(
        serializedIdentity,
        { skipValidation: true },
      );

      const receivedIdentityWithoutBalance = receivedIdentity.toJSON();
      delete receivedIdentityWithoutBalance.balance;

      const localIdentityWithoutBalance = identity.toJSON();
      delete localIdentityWithoutBalance.balance;

      expect(receivedIdentityWithoutBalance).to.deep.equal(localIdentityWithoutBalance);
      expect(receivedIdentity.getBalance()).to.be.greaterThan(0);
    });

    it('should be able to get newly created identity id by it\'s first public key', async () => {
      const [identityId] = await client.getDAPIClient().platform.getIdentityIdsByPublicKeyHashes(
        [identity.getPublicKeyById(0).hash()],
      );

      expect(identityId).to.be.not.null();
      expect(identityId).to.deep.equal(identity.getId());
    });

    describe('Credits', () => {
      let dataContractFixture;

      before(async () => {
        dataContractFixture = getDataContractFixture(identity.getId());

        await client.platform.contracts.broadcast(dataContractFixture, identity);

        client.getApps().set('customContracts', {
          contractId: dataContractFixture.getId(),
          contract: dataContractFixture,
        });
      });

      it('should fail to create more documents if there are no more credits', async () => {
        const document = await client.platform.documents.create(
          'customContracts.niceDocument',
          identity,
          {
            name: 'Some Very Long Long Long Name'.repeat(100),
          },
        );

        try {
          await client.platform.documents.broadcast({
            create: [document],
          }, identity);

          expect.fail('Error was not thrown');
        } catch (e) {
          expect(e.details).to.equal('Failed precondition: Not enough credits');
        }
      });

      it.skip('should fail top-up if instant lock is not valid', async () => {
        await waitForBalanceToChange(walletAccount);

        const {
          transaction,
          privateKey,
          outputIndex,
        } = await createAssetLockTransaction({
          client,
        }, 1);

        const instantLock = createFakeInstantLock(transaction.hash);
        const assetLockProof = await dpp.identity.createInstantAssetLockProof(instantLock);

        const identityTopUpTransition = dpp.identity.createIdentityTopUpTransition(
          identity.getId(),
          transaction,
          outputIndex,
          assetLockProof,
        );
        identityTopUpTransition.signByPrivateKey(
          privateKey,
        );

        try {
          await client.getDAPIClient().platform.broadcastStateTransition(
            identityTopUpTransition.toBuffer(),
          );

          expect.fail('Error was not thrown');
        } catch (e) {
          const [error] = JSON.parse(e.metadata.get('errors'));
          expect(error.name).to.equal('IdentityAssetLockTransactionNotFoundError');
        }
      });

      it('should be able to top-up credit balance', async () => {
        await waitForBalanceToChange(walletAccount);

        const identityBeforeTopUp = await client.platform.identities.get(
          identity.getId(),
        );
        const balanceBeforeTopUp = identityBeforeTopUp.getBalance();
        const topUpAmount = 100;
        const topUpCredits = topUpAmount * 1000;

        await client.platform.identities.topUp(identity.getId(), topUpAmount);

        await waitForBalanceToChange(walletAccount);

        const identityAfterTopUp = await client.platform.identities.get(
          identity.getId(),
        );

        expect(identityAfterTopUp.getBalance()).to.be.greaterThan(balanceBeforeTopUp);
        expect(identityAfterTopUp.getBalance()).to.be.lessThan(balanceBeforeTopUp + topUpCredits);
      });

      it('should be able to create more documents after the top-up', async () => {
        const document = await client.platform.documents.create(
          'customContracts.niceDocument',
          identity,
          {
            name: 'Some Very Long Long Long Name',
          },
        );

        await client.platform.documents.broadcast({
          create: [document],
        }, identity);
      });
    });
  });
});
