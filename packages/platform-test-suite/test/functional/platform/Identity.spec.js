const DashPlatformProtocol = require('@dashevo/dpp');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const {
  PublicKey,
} = require('@dashevo/dashcore-lib');

const waitForBlocks = require('../../../lib/waitForBlocks');
const waitForBalanceToChange = require('../../../lib/test/waitForBalanceToChange');

const createOutPointTx = require('../../../lib/test/createOutPointTx');
const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Platform', () => {
  describe('Identity', () => {
    let dpp;
    let client;
    let walletAccount;
    let identityCreateTransition;
    let identity;
    let identityRawPublicKey;
    let walletPublicKey;
    let walletPrivateKey;

    before(async () => {
      dpp = new DashPlatformProtocol();

      client = await createClientWithFundedWallet();
      walletAccount = await client.getWalletAccount();
      ({
        publicKey: walletPublicKey,
        privateKey: walletPrivateKey,
      } = walletAccount.getIdentityHDKeyByIndex(0, 0));
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should fail to create an identity if outpoint was not found', async () => {
      identity = dpp.identity.create(
        Buffer.alloc(36),
        [walletPublicKey],
      );

      identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);
      identityCreateTransition.signByPrivateKey(
        walletPrivateKey,
      );

      try {
        await client.getDAPIClient().platform.broadcastStateTransition(
          identityCreateTransition.serialize(),
        );
        expect.fail('Error was not thrown');
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors'));
        expect(error.name).to.equal('IdentityAssetLockTransactionNotFoundError');
      }
    });

    it('should create an identity', async () => {
      identity = await client.platform.identities.register(2);
      identityRawPublicKey = new PublicKey(
        Buffer.from(identity.getPublicKeys()[0].getData(), 'base64'),
      );

      await waitForBalanceToChange(walletAccount);
    });

    it('should fail to create an identity with the same first public key', async () => {
      const {
        transaction,
        privateKey,
      } = createOutPointTx(
        1,
        walletAccount,
        walletPublicKey,
        walletPrivateKey,
      );

      const outPoint = transaction.getOutPointBuffer(0);

      await client.getDAPIClient().core.broadcastTransaction(transaction.toBuffer());
      await waitForBlocks(client.getDAPIClient(), 1);

      const otherIdentity = dpp.identity.create(
        outPoint,
        [walletPublicKey],
      );

      const otherIdentityCreateTransition = dpp.identity.createIdentityCreateTransition(
        otherIdentity,
      );
      otherIdentityCreateTransition.signByPrivateKey(
        privateKey,
      );

      try {
        await client.getDAPIClient().platform.broadcastStateTransition(
          otherIdentityCreateTransition.serialize(),
        );

        expect.fail('Error was not thrown');
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors'));
        expect(error.name).to.equal('IdentityFirstPublicKeyAlreadyExistsError');
        expect(error.publicKeyHash).to.equal(identity.getPublicKeyById(0).hash());
      }
    });

    it('should be able to get newly created identity', async () => {
      const fetchedIdentity = await client.platform.identities.get(
        identity.getId(),
      );

      expect(fetchedIdentity).to.be.not.null();
      expect(fetchedIdentity.toJSON()).to.deep.equal({
        ...identity.toJSON(),
        balance: 1826,
      });

      // updating balance
      identity.setBalance(fetchedIdentity.getBalance());
    });

    it('should be able to get newly created identity by it\'s first public key', async () => {
      const serializedIdentity = await client.getDAPIClient().platform.getIdentityByFirstPublicKey(
        identity.getPublicKeyById(0).hash(),
      );

      expect(serializedIdentity).to.be.not.null();

      const receivedIdentity = dpp.identity.createFromSerialized(
        serializedIdentity,
        { skipValidation: true },
      );

      expect(receivedIdentity.toJSON()).to.deep.equal({
        ...identity.toJSON(),
        balance: 1826,
      });
    });

    it('should be able to get newly created identity id by it\'s first public key', async () => {
      const identityId = await client.getDAPIClient().platform.getIdentityIdByFirstPublicKey(
        identity.getPublicKeyById(0).hash(),
      );

      expect(identityId).to.be.not.null();
      expect(identityId).to.equal(identity.getId());
    });

    describe('Credits', () => {
      let dataContractFixture;

      before(async () => {
        dataContractFixture = getDataContractFixture(identity.getId());

        await client.platform.contracts.broadcast(dataContractFixture, identity);

        client.apps.customContracts = {
          contractId: dataContractFixture.getId(),
          contract: dataContractFixture,
        };
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

      it('should fail top-up if transaction has not been sent', async () => {
        await waitForBalanceToChange(walletAccount);

        const {
          transaction,
          privateKey,
        } = createOutPointTx(
          1,
          walletAccount,
          identityRawPublicKey,
          walletPrivateKey,
        );

        const outPoint = transaction.getOutPointBuffer(0);

        const identityTopUpTransition = dpp.identity.createIdentityTopUpTransition(
          identity.getId(),
          outPoint,
        );
        identityTopUpTransition.signByPrivateKey(
          privateKey,
        );

        try {
          await client.getDAPIClient().platform.broadcastStateTransition(
            identityTopUpTransition.serialize(),
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
