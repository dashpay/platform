const crypto = require('crypto');

const DashPlatformProtocol = require('@dashevo/dpp');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const { createFakeInstantLock } = require('dash/build/src/utils/createFakeIntantLock');
const { default: createAssetLockProof } = require('dash/build/src/SDK/Client/Platform/methods/identities/internal/createAssetLockProof');
const { default: createIdentityCreateTransition } = require('dash/build/src/SDK/Client/Platform/methods/identities/internal/createIdentityCreateTransition');
const { default: createIdentityTopUpTransition } = require('dash/build/src/SDK/Client/Platform/methods/identities/internal/createIdnetityTopUpTransition');
const { default: createAssetLockTransaction } = require('dash/build/src/SDK/Client/Platform/createAssetLockTransaction');
const { default: waitForCoreChainLockedHeight } = require('dash/build/src/SDK/Client/Platform/methods/identities/internal/waitForCoreChainLockedHeight');

const { StateTransitionBroadcastError } = require('dash/build/src/errors/StateTransitionBroadcastError');
const InvalidInstantAssetLockProofSignatureError = require('@dashevo/dpp/lib/errors/consensus/basic/identity/InvalidInstantAssetLockProofSignatureError');
const IdentityAssetLockTransactionOutPointAlreadyExistsError = require('@dashevo/dpp/lib/errors/consensus/basic/identity/IdentityAssetLockTransactionOutPointAlreadyExistsError');
const BalanceIsNotEnoughError = require('@dashevo/dpp/lib/errors/consensus/fee/BalanceIsNotEnoughError');

const InvalidIdentityKeySignatureError = require('@dashevo/dpp/lib/errors/consensus/basic/identity/InvalidIdentityKeySignatureError');

const DAPIClient = require('@dashevo/dapi-client/lib/DAPIClient');
const { hash } = require('@dashevo/dpp/lib/util/hash');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');
const Transaction = require('@dashevo/dashcore-lib/lib/transaction');

const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const Identity = require('@dashevo/dpp/lib/identity/Identity');
const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');
const wait = require('../../../lib/wait');
const getDAPISeeds = require('../../../lib/test/getDAPISeeds');

describe('Platform', () => {
  describe('Identity', () => {
    let dpp;
    let client;
    let identity;
    let walletAccount;

    before(async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      client = await createClientWithFundedWallet();

      const account = await client.platform.client.getWalletAccount();
      const identityIndex = await account.getUnusedIdentityIndex();

      const { privateKey: identityPrivateKey } = account
        .identities
        .getIdentityHDKeyByIndex(identityIndex, 1);

      walletAccount = await client.getWalletAccount();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should create an identity', async () => {
      identity = await client.platform.identities.register(50);

      expect(identity).to.exist();
    });

    it('should fail to create an identity if instantLock is not valid', async () => {
      const {
        transaction,
        privateKey,
        outputIndex,
      } = await createAssetLockTransaction({
        client,
      }, 1);

      const invalidInstantLock = createFakeInstantLock(transaction.hash);
      const assetLockProof = await dpp.identity.createInstantAssetLockProof(
        invalidInstantLock,
        transaction,
        outputIndex,
      );

      const {
        identityCreateTransition: invalidIdentityCreateTransition,
      } = await createIdentityCreateTransition(
        client.platform, assetLockProof, privateKey,
      );

      let broadcastError;

      try {
        await client.platform.broadcastStateTransition(
          invalidIdentityCreateTransition,
        );
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause()).to.be.an.instanceOf(
        InvalidInstantAssetLockProofSignatureError,
      );
    });

    it('should fail to create an identity with already used asset lock output', async () => {
      const {
        transaction,
        privateKey,
        outputIndex,
      } = await createAssetLockTransaction({ client }, 1);

      await client.getDAPIClient().core.broadcastTransaction(transaction.toBuffer());

      const assetLockProof = await createAssetLockProof(client.platform, transaction, outputIndex);

      // Creating normal transition
      const {
        identity: identityOne,
        identityCreateTransition: identityCreateTransitionOne,
        identityIndex: identityOneIndex,
      } = await createIdentityCreateTransition(
        client.platform, assetLockProof, privateKey,
      );

      await client.platform.broadcastStateTransition(
        identityCreateTransitionOne,
      );

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      walletAccount.storage
        .getWalletStore(walletAccount.walletId)
        .insertIdentityIdAtIndex(
          identityOne.getId().toString(),
          identityOneIndex,
        );

      // Creating transition that tries to spend the same transaction
      const {
        identityCreateTransition: identityCreateDoubleSpendTransition,
      } = await createIdentityCreateTransition(
        client.platform, assetLockProof, privateKey,
      );

      let broadcastError;

      try {
        await client.platform.broadcastStateTransition(
          identityCreateDoubleSpendTransition,
        );
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause()).to.be.an.instanceOf(
        IdentityAssetLockTransactionOutPointAlreadyExistsError,
      );
    });

    it('should not be able to create an identity without key proof', async () => {
      const {
        transaction,
        privateKey,
        outputIndex,
      } = await createAssetLockTransaction({ client }, 1);

      await client.getDAPIClient().core.broadcastTransaction(transaction.toBuffer());

      const assetLockProof = await createAssetLockProof(client.platform, transaction, outputIndex);

      // Creating normal transition
      const {
        identityCreateTransition,
      } = await createIdentityCreateTransition(
        client.platform, assetLockProof, privateKey,
      );

      // Remove signature

      const [masterKey] = identityCreateTransition.getPublicKeys();
      masterKey.setSignature(crypto.randomBytes(65));

      // Broadcast

      let broadcastError;

      try {
        await client.platform.broadcastStateTransition(
          identityCreateTransition,
          { skipValidation: true },
        );
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause()).to.be.an.instanceOf(
        InvalidIdentityKeySignatureError,
      );
    });

    it('should be able to get newly created identity', async () => {
      const fetchedIdentity = await client.platform.identities.get(
        identity.getId(),
      );

      expect(fetchedIdentity).to.be.not.null();

      const fetchedIdentityWithoutBalance = fetchedIdentity.toObject();
      delete fetchedIdentityWithoutBalance.balance;

      const localIdentityWithoutBalance = identity.toObject();
      delete localIdentityWithoutBalance.balance;

      expect(fetchedIdentityWithoutBalance).to.deep.equal(localIdentityWithoutBalance);

      expect(fetchedIdentity.getBalance()).to.be.greaterThan(0);
    });

    it('should be able to get newly created identity by it\'s first public key', async () => {
      const response = await client.getDAPIClient().platform
        .getIdentitiesByPublicKeyHashes(
          [identity.getPublicKeyById(0).hash()],
        );

      const [[serializedIdentity]] = response.getIdentities();

      expect(serializedIdentity).to.be.not.null();

      const receivedIdentity = dpp.identity.createFromBuffer(
        serializedIdentity,
        { skipValidation: true },
      );

      const receivedIdentityWithoutBalance = receivedIdentity.toObject();
      delete receivedIdentityWithoutBalance.balance;

      const localIdentityWithoutBalance = identity.toObject();
      delete localIdentityWithoutBalance.balance;

      expect(receivedIdentityWithoutBalance).to.deep.equal(localIdentityWithoutBalance);
      expect(receivedIdentity.getBalance()).to.be.greaterThan(0);
    });

    it('should be able to get newly created identity id by it\'s first public key', async () => {
      const response = await client.getDAPIClient().platform.getIdentityIdsByPublicKeyHashes(
        [identity.getPublicKeyById(0).hash()],
      );

      const [[identityId]] = response.getIdentityIds();

      expect(identityId).to.be.not.null();
      expect(identityId).to.deep.equal(identity.getId());
    });

    describe('chainLock', function describe() {
      let chainLockIdentity;

      this.timeout(850000);

      it('should create identity using chainLock', async () => {
        const {
          transaction,
          privateKey,
          outputIndex,
        } = await createAssetLockTransaction({
          client,
        }, 100);

        // Broadcast Asset Lock transaction
        await client.getDAPIClient().core.broadcastTransaction(transaction.toBuffer());

        // Wait for transaction to be mined and chain locked
        const { promise: metadataPromise } = walletAccount.waitForTxMetadata(transaction.id);

        const { height: transactionHeight } = await metadataPromise;

        const outPoint = transaction.getOutPointBuffer(outputIndex);
        const assetLockProof = await dpp.identity.createChainAssetLockProof(
          transactionHeight,
          outPoint,
        );

        // Wait for platform chain to sync core height up to transaction height
        const {
          promise: coreHeightPromise,
        } = await waitForCoreChainLockedHeight(client.platform, transactionHeight);

        await coreHeightPromise;

        const identityCreateTransitionData = await createIdentityCreateTransition(
          client.platform, assetLockProof, privateKey,
        );

        const {
          identityCreateTransition,
        } = identityCreateTransitionData;

        ({ identity: chainLockIdentity } = identityCreateTransitionData);

        await client.platform.broadcastStateTransition(
          identityCreateTransition,
        );

        // Additional wait time to mitigate testnet latency
        if (process.env.NETWORK === 'testnet') {
          await wait(5000);
        }
      });

      it('should be able to get newly created identity', async () => {
        const fetchedIdentity = await client.platform.identities.get(
          chainLockIdentity.getId(),
        );

        expect(fetchedIdentity).to.be.not.null();

        const fetchedIdentityWithoutBalance = fetchedIdentity.toObject();
        delete fetchedIdentityWithoutBalance.balance;

        const localIdentityWithoutBalance = chainLockIdentity.toObject();
        delete localIdentityWithoutBalance.balance;

        expect(fetchedIdentityWithoutBalance).to.deep.equal(localIdentityWithoutBalance);

        expect(fetchedIdentity.getBalance()).to.be.greaterThan(0);
      });
    });

    describe('Credits', () => {
      let dataContractFixture;

      before(async () => {
        dataContractFixture = getDataContractFixture(identity.getId());

        await client.platform.contracts.publish(dataContractFixture, identity);

        // Additional wait time to mitigate testnet latency
        if (process.env.NETWORK === 'testnet') {
          await wait(5000);
        }

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

        let broadcastError;

        try {
          await client.platform.documents.broadcast({
            create: [document],
          }, identity);
        } catch (e) {
          broadcastError = e;
        }

        expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
        expect(broadcastError.getCause()).to.be.an.instanceOf(
          BalanceIsNotEnoughError,
        );
      });

      it.skip('should fail top-up if instant lock is not valid', async () => {
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
        await identityTopUpTransition.signByPrivateKey(
          privateKey,
          IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        );

        let broadcastError;

        try {
          await client.platform.broadcastStateTransition(
            identityTopUpTransition,
          );
        } catch (e) {
          broadcastError = e;
        }

        expect(broadcastError).to.exist();
        expect(broadcastError.message).to.be.equal('State Transition is invalid: InvalidIdentityAssetLockProofSignatureError: Invalid Asset lock proof signature');
        expect(broadcastError.code).to.be.equal(3);
        const [error] = broadcastError.data.errors;
        expect(error.name).to.equal('IdentityAssetLockTransactionNotFoundError');
      });

      it('should be able to top-up credit balance', async () => {
        const identityBeforeTopUp = await client.platform.identities.get(
          identity.getId(),
        );
        const balanceBeforeTopUp = identityBeforeTopUp.getBalance();
        const topUpAmount = 100;
        const topUpCredits = topUpAmount * 1000;

        await client.platform.identities.topUp(identity.getId(), topUpAmount);

        // Additional wait time to mitigate testnet latency
        if (process.env.NETWORK === 'testnet') {
          await wait(5000);
        }

        const identityAfterTopUp = await client.platform.identities.get(
          identity.getId(),
        );

        expect(identityAfterTopUp.getBalance()).to.be.greaterThan(balanceBeforeTopUp);

        // TODO: Temporary changed to equal since we are not deducting fees from balance atm
        // expect(identityAfterTopUp.getBalance()).to.be
        //   .lessThan(balanceBeforeTopUp + topUpCredits);
        expect(identityAfterTopUp.getBalance()).to.be.equal(balanceBeforeTopUp + topUpCredits);
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

        // Additional wait time to mitigate testnet latency
        if (process.env.NETWORK === 'testnet') {
          await wait(5000);
        }
      });

      it('should fail to top up an identity with already used asset lock output', async () => {
        const {
          transaction,
          privateKey,
          outputIndex,
        } = await createAssetLockTransaction({ client }, 1);

        await client.getDAPIClient().core.broadcastTransaction(transaction.toBuffer());

        const assetLockProof = await createAssetLockProof(
          client.platform,
          transaction,
          outputIndex,
        );

        // Creating normal transition
        const identityTopUpTransitionOne = await createIdentityTopUpTransition(
          client.platform, assetLockProof, privateKey, identity.getId(),
        );
        // Creating ST that tries to spend the same output
        const conflictingTopUpStateTransition = await createIdentityTopUpTransition(
          client.platform, assetLockProof, privateKey, identity.getId(),
        );

        await client.platform.broadcastStateTransition(
          identityTopUpTransitionOne,
        );

        // Additional wait time to mitigate testnet latency
        if (process.env.NETWORK === 'testnet') {
          await wait(5000);
        }

        let broadcastError;

        try {
          await client.platform.broadcastStateTransition(
            conflictingTopUpStateTransition,
          );
        } catch (e) {
          broadcastError = e;
        }

        expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
        expect(broadcastError.getCause()).to.be.an.instanceOf(
          IdentityAssetLockTransactionOutPointAlreadyExistsError,
        );
      });
    });

    describe('Update', () => {
      it('should be able to add public key to the identity', async () => {
        const identityBeforeUpdate = new Identity(identity.toObject());

        expect(identityBeforeUpdate.getPublicKeyById(2)).to.not.exist();

        const account = await client.platform.client.getWalletAccount();
        const identityIndex = await account.getUnusedIdentityIndex();

        const { privateKey: identityPrivateKey } = account
          .identities
          .getIdentityHDKeyByIndex(identityIndex, 1);

        const identityPublicKey = identityPrivateKey.toPublicKey().toBuffer();

        const newPublicKey = new IdentityPublicKey(
          {
            id: 2,
            type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
            purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
            securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
            data: identityPublicKey,
            readOnly: false,
          },
        );

        const update = {
          add: [newPublicKey],
        };

        await client.platform.identities.update(
          identity,
          update,
          {
            [newPublicKey.getId()]: identityPrivateKey,
          },
        );

        identity = await client.platform.identities.get(
          identity.getId(),
        );

        expect(identity.getRevision()).to.equal(identityBeforeUpdate.getRevision() + 1);
        expect(identity.getPublicKeyById(2)).to.exist();

        expect(identity.getPublicKeyById(2).toObject()).to.deep.equal(
          newPublicKey.toObject(),
        );
      });

      it('should be able to disable public key of the identity', async () => {
        const now = new Date().getTime();

        const identityBeforeUpdate = new Identity(identity.toObject());

        const publicKeyToDisable = identityBeforeUpdate.getPublicKeyById(2);
        const update = {
          disable: [publicKeyToDisable],
        };

        await client.platform.identities.update(
          identity,
          update,
        );

        identity = await client.platform.identities.get(
          identity.getId(),
        );

        expect(identity.getRevision()).to.equal(identityBeforeUpdate.getRevision() + 1);
        expect(identity.getPublicKeyById(2)).to.exist();
        expect(identity.getPublicKeyById(2).getDisabledAt()).to.be.at.least(now);

        expect(identity.getPublicKeyById(0)).to.exist();
        expect(identity.getPublicKeyById(0).getDisabledAt()).to.be.undefined();
      });
    });

    describe('Masternodes', () => {
      let dapiClient;
      const network = process.env.NETWORK;

      beforeEach(() => {
        dapiClient = new DAPIClient({
          network,
          seeds: getDAPISeeds(),
        });
      });

      it('should receive masternode identities', async () => {
        const bestBlockHash = await dapiClient.core.getBestBlockHash();
        const baseBlockHash = await dapiClient.core.getBlockHash(1);

        const { mnList } = await dapiClient.core.getMnListDiff(
          baseBlockHash,
          bestBlockHash,
        );

        for (const masternodeEntry of mnList) {
          const masternodeIdentityId = Identifier.from(
            Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
          );

          let fetchedIdentity = await client.platform.identities.get(
            masternodeIdentityId,
          );

          expect(fetchedIdentity).to.be.not.null();

          const { transaction: transactionBuffer } = await client.dapiClient.core.getTransaction(
            masternodeEntry.proRegTxHash,
          );

          const transaction = new Transaction(transactionBuffer);

          if (transaction.extraPayload.operatorReward > 0) {
            const operatorPubKey = Buffer.from(masternodeEntry.pubKeyOperator, 'hex');

            const operatorIdentityHash = hash(
              Buffer.concat([
                Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
                operatorPubKey,
              ]),
            );

            const operatorIdentityId = Identifier.from(operatorIdentityHash);

            fetchedIdentity = await client.platform.identities.get(
              operatorIdentityId,
            );

            expect(fetchedIdentity).to.be.not.null();
          }
        }
      });
    });
  });
});
