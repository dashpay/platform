const Dash = require('dash');

const { createFakeInstantLock } = require('dash/build/utils/createFakeIntantLock');

const { hash, sha256 } = require('@dashevo/wasm-dpp/lib/utils/hash');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../../lib/waitForSTPropagated');

const {
  Essentials: {
    Buffer,
  },
  Core: {
    Transaction,
  },
  Errors: {
    StateTransitionBroadcastError,
  },
  PlatformProtocol: {
    Identifier,
    IdentityPublicKey,
    InvalidInstantAssetLockProofSignatureError,
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
    BasicECDSAError,
    IdentityPublicKeyWithWitness,
    IdentityInsufficientBalanceError,
  },
} = Dash;

describe('Platform', () => {
  describe('Identity', function describeIdentity() {
    this.bail(true); // bail on first failure

    let client;
    let identity;
    let walletAccount;

    before(async () => {
      client = await createClientWithFundedWallet(20000000);

      walletAccount = await client.getWalletAccount();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should create an identity', async () => {
      identity = await client.platform.identities.register(400000);

      expect(identity).to.exist();
    });

    it('should fail to create an identity if asset lock amount is less than minimal', async () => {
      let broadcastError;

      try {
        await client.platform.identities.register(197000);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause().getCode()).to.equal(10530);
      expect(broadcastError.getCause()).to.be.an.instanceOf(
        IdentityAssetLockTransactionOutPointNotEnoughBalanceError,
      );
    });

    it('should fail to create an identity if instantLock is not valid', async () => {
      await client.platform.initialize();

      const {
        transaction,
        privateKey,
        outputIndex,
      } = await client.platform.identities.utils.createAssetLockTransaction(200000);

      const invalidInstantLock = createFakeInstantLock(transaction.hash);
      const assetLockProof = await client.platform.dpp.identity.createInstantAssetLockProof(
        invalidInstantLock.toBuffer(),
        transaction.toBuffer(),
        outputIndex,
      );

      const {
        identityCreateTransition: invalidIdentityCreateTransition,
      } = await client.platform.identities
        .utils.createIdentityCreateTransition(assetLockProof, privateKey);

      let broadcastError;

      try {
        await client.platform.broadcastStateTransition(
          invalidIdentityCreateTransition,
        );
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause().getCode()).to.equal(10513);
      expect(broadcastError.getCause()).to.be.an.instanceOf(
        InvalidInstantAssetLockProofSignatureError,
      );
    });

    it('should fail to create an identity with already used asset lock output', async () => {
      // Create new identity
      const sourceIdentity = await client.platform.identities.register(400000);

      const {
        transaction,
        privateKey,
        outputIndex,
      } = await client.platform.identities.utils.createAssetLockTransaction(200000);

      const account = await client.getWalletAccount();

      // Do not `await` for this call, it does not propagate instant locks
      // and createAsstLockProof falls back to chain lock instead which is not what we want
      account.broadcastTransaction(transaction);

      // Creating normal transition
      const assetLockProof = await client.platform.identities.utils
        .createAssetLockProof(transaction, outputIndex);

      // Top up identity
      const identityTopUpTransition = await client.platform.identities.utils
        .createIdentityTopUpTransition(assetLockProof, privateKey, sourceIdentity.getId());

      await client.platform.broadcastStateTransition(
        identityTopUpTransition,
      );

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      // Try to create transition that tries to spend the same transaction
      const {
        identityCreateTransition: identityCreateDoubleSpendTransition,
      } = await client.platform.identities.utils
        .createIdentityCreateTransition(assetLockProof, privateKey);

      let broadcastError;

      try {
        await client.platform.broadcastStateTransition(
          identityCreateDoubleSpendTransition,
        );
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause().getCode()).to.equal(10504);
      expect(broadcastError.getCause()).to.be.an.instanceOf(
        IdentityAssetLockTransactionOutPointAlreadyExistsError,
      );
    });

    it('should not be able to create an identity without key proof', async () => {
      const {
        transaction,
        privateKey,
        outputIndex,
      } = await client.platform.identities.utils.createAssetLockTransaction(210000);

      const account = await client.getWalletAccount();

      // Do not `await` for this call, it does not propagate instant locks
      // and createAsstLockProof falls back to chain lock instead which is not what we want
      account.broadcastTransaction(transaction);

      // Creating normal transition
      const assetLockProof = await client.platform.identities.utils.createAssetLockProof(
        transaction,
        outputIndex,
      );

      const {
        identityCreateTransition,
      } = await client.platform.identities
        .utils.createIdentityCreateTransition(assetLockProof, privateKey);

      // Remove signature

      const keys = identityCreateTransition.getPublicKeys();
      const [masterKey] = keys;
      masterKey.setSignature(Buffer.alloc(65));
      identityCreateTransition.setPublicKeys(keys);

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
      expect(broadcastError.getCause().getCode()).to.equal(20009);
      expect(broadcastError.getCause()).to.be.an.instanceOf(
        BasicECDSAError,
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

    it('should be able to get newly created identity by it\'s public key', async () => {
      const response = await client.getDAPIClient().platform.getIdentityByPublicKeyHash(
        identity.getPublicKeyById(0).hash(),
      );

      const fetchedIdentity = response.getIdentity();

      expect(fetchedIdentity.length).to.be.greaterThan(0);

      // TODO(rs-drive-abci): fix. rs-drive-abci now only returning identity bytes without the
      //   asset lock proof. We would also want to do the same in rs-dpp and wasm-dpp, but
      //   we can't right now because of the backward compatibility.
      const bytesToCheck = fetchedIdentity.slice(0, fetchedIdentity.length - 3);
      expect(identity.toBuffer().includes(bytesToCheck)).to.be.true();
    });

    describe('chainLock', function describe() {
      let chainLockIdentity;

      this.timeout(850000);

      it('should create identity using chainLock', async () => {
        await client.platform.initialize();

        // Broadcast Asset Lock transaction
        const {
          transaction,
          privateKey,
          outputIndex,
        } = await client.platform.identities.utils.createAssetLockTransaction(200000);

        const account = await client.getWalletAccount();

        await account.broadcastTransaction(transaction);

        // Wait for transaction to be mined and chain locked
        const { promise: metadataPromise } = walletAccount.waitForTxMetadata(transaction.id);

        const { height: transactionHeight } = await metadataPromise;

        const assetLockProof = await client.platform.dpp.identity.createChainAssetLockProof(
          transactionHeight,
          transaction.getOutPointBuffer(outputIndex),
        );

        // Wait for platform chain to sync core height up to transaction height
        const {
          promise: coreHeightPromise,
        } = await client.platform.identities.utils
          .waitForCoreChainLockedHeight(transactionHeight);

        await coreHeightPromise;

        const identityCreateTransitionData = await client.platform.identities.utils
          .createIdentityCreateTransition(assetLockProof, privateKey);

        const {
          identityCreateTransition,
        } = identityCreateTransitionData;

        ({ identity: chainLockIdentity } = identityCreateTransitionData);

        await client.platform.broadcastStateTransition(
          identityCreateTransition,
        );

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();
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
        const nextNonce = await client.platform.nonceManager
          .bumpIdentityNonce(identity.getId());
        dataContractFixture = await getDataContractFixture(nextNonce, identity.getId());

        await client.platform.contracts.publish(dataContractFixture, identity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        client.getApps().set('customContracts', {
          contractId: dataContractFixture.getId(),
          contract: dataContractFixture,
        });
      });

      it('should fail to create more documents if there are no more credits', async () => {
        const lowBalanceIdentity = await client.platform.identities.register(200000);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        const document = await client.platform.documents.create(
          'customContracts.niceDocument',
          lowBalanceIdentity,
          {
            name: 'Some Very Long Long Long Name'.repeat(100),
          },
        );

        let broadcastError;

        try {
          await client.platform.documents.broadcast({
            create: [document],
          }, lowBalanceIdentity);
        } catch (e) {
          broadcastError = e;
        }

        expect(broadcastError).to.exist();
        expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
        expect(broadcastError.getCause()).to.be.an.instanceOf(
          IdentityInsufficientBalanceError,
        );
      });

      it('should fail top-up if instant lock is not valid', async () => {
        const {
          transaction,
          privateKey,
          outputIndex,
        } = await client.platform.identities.utils.createAssetLockTransaction(200000);

        const instantLock = createFakeInstantLock(transaction.hash);
        const assetLockProof = await client.platform.dpp.identity
          .createInstantAssetLockProof(
            instantLock.toBuffer(),
            transaction.toBuffer(),
            outputIndex,
          );

        const identityTopUpTransition = client.platform.dpp.identity
          .createIdentityTopUpTransition(
            identity.getId(),
            assetLockProof,
          );
        await identityTopUpTransition.signByPrivateKey(
          privateKey.toBuffer(),
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
        expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
        expect(broadcastError.getCause()).to.be.an.instanceOf(
          InvalidInstantAssetLockProofSignatureError,
        );
      });

      it('should be able to top-up credit balance', async () => {
        const identityBeforeTopUp = await client.platform.identities.get(
          identity.getId(),
        );
        const balanceBeforeTopUp = identityBeforeTopUp.getBalance();
        const topUpAmount = 1000000;
        const topUpCredits = topUpAmount * 1000;

        await client.platform.identities.topUp(identity.getId(), topUpAmount);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        const identityAfterTopUp = await client.platform.identities.get(
          identity.getId(),
        );

        expect(identityAfterTopUp.getBalance()).to.be.greaterThan(balanceBeforeTopUp);

        expect(identityAfterTopUp.getBalance()).to.be
          .lessThan(balanceBeforeTopUp + topUpCredits);
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
        await waitForSTPropagated();
      });

      it('should fail to top up an identity with already used asset lock output', async () => {
        const {
          transaction,
          privateKey,
          outputIndex,
        } = await client.platform.identities.utils.createAssetLockTransaction(200000);

        const account = await client.getWalletAccount();

        await account.broadcastTransaction(transaction);

        // Creating normal transition
        const assetLockProof = await client.platform.identities.utils.createAssetLockProof(
          transaction,
          outputIndex,
        );

        const identityTopUpTransitionOne = await client.platform.identities.utils
          .createIdentityTopUpTransition(assetLockProof, privateKey, identity.getId());

        // Creating ST that tries to spend the same output

        const anotherIdentity = await client.platform.identities.register(200000);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        const conflictingTopUpStateTransition = await client.platform.identities.utils
          .createIdentityTopUpTransition(assetLockProof, privateKey, anotherIdentity.getId());

        await client.platform.broadcastStateTransition(
          identityTopUpTransitionOne,
        );

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

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

      describe('Credit transfer', () => {
        let recipient;
        before(async () => {
          recipient = await client.platform.identities.register(400000);

          // Additional wait time to mitigate testnet latency
          await waitForSTPropagated();
        });

        it('should be able to transfer credits from one identity to another', async () => {
          const identityBeforeTransfer = await client.platform.identities.get(
            identity.getId(),
          );

          const recipientBeforeTransfer = await client.platform.identities.get(
            recipient.getId(),
          );

          const transferAmount = 300000;

          await client.platform.identities.creditTransfer(
            identityBeforeTransfer,
            recipient.getId(),
            transferAmount,
          );

          // Additional wait time to mitigate testnet latency
          await waitForSTPropagated();

          const identityAfterTransfer = await client.platform.identities.get(
            identity.getId(),
          );

          const recipientAfterTransfer = await client.platform.identities.get(
            recipient.getId(),
          );

          const identityBalanceBefore = identityBeforeTransfer.getBalance();
          const identityBalanceAfter = identityAfterTransfer.getBalance();

          const recipientBalanceBefore = recipientBeforeTransfer.getBalance();
          const recipientBalanceAfter = recipientAfterTransfer.getBalance();

          expect(recipientBalanceAfter).to.be.equal(recipientBalanceBefore + transferAmount);

          // TODO: implement the way to get the fee
          expect(identityBalanceAfter).to.be.lessThan(identityBalanceBefore + transferAmount);
        });

        it('should not be able to transfer more credits then have', async () => {
          identity = await client.platform.identities.get(
            identity.getId(),
          );

          let transferError;

          try {
            await client.platform.identities.creditTransfer(
              identity,
              recipient.getId(),
              identity.getBalance() + 1,
            );
          } catch (e) {
            transferError = e;
          }

          expect(transferError).to.exist();
          expect(transferError).to.be.an.instanceOf(StateTransitionBroadcastError);
          expect(transferError.getCause()).to.be.an.instanceOf(IdentityInsufficientBalanceError);
          expect(transferError.getCause().getIdentityId()).to.deep.equal(identity.getId());
        });
      });
    });

    describe('Update', () => {
      it('should be able to add public key to the identity', async () => {
        const identityBeforeUpdate = identity.toObject();

        const nextKeyId = identityBeforeUpdate.publicKeys.length;

        const account = await client.platform.client.getWalletAccount();
        const identityIndex = await account.getUnusedIdentityIndex();

        const { privateKey: identityPrivateKey } = account
          .identities
          .getIdentityHDKeyByIndex(identityIndex, 1);

        const identityPublicKey = identityPrivateKey.toPublicKey().toBuffer();

        const newPublicKey = new IdentityPublicKeyWithWitness(1);
        newPublicKey.setId(nextKeyId);
        newPublicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);
        newPublicKey.setData(identityPublicKey);

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

        await waitForSTPropagated();

        identity = await client.platform.identities.get(
          identity.getId(),
        );

        expect(identity.getRevision()).to.equal(identityBeforeUpdate.revision + 1);
        expect(identity.getPublicKeyById(2)).to.exist();

        const newPublicKeyObject = newPublicKey.toObject(true);
        const expectedPublicKey = identity.getPublicKeyById(4).toObject(true);
        delete expectedPublicKey.disabledAt;
        expect(expectedPublicKey).to.deep.equal(
          newPublicKeyObject,
        );
      });

      it('should be able to disable public key of the identity', async () => {
        const now = new Date();

        const identityBeforeUpdate = identity.toObject();

        const publicKeyToDisable = identity.getPublicKeyById(2);
        const update = {
          disable: [publicKeyToDisable],
        };

        await client.platform.identities.update(
          identity,
          update,
        );

        await waitForSTPropagated();

        identity = await client.platform.identities.get(
          identity.getId(),
        );

        expect(identity.getRevision()).to.equal(identityBeforeUpdate.revision + 1);
        expect(identity.getPublicKeyById(2)).to.exist();
        expect(identity.getPublicKeyById(2).getDisabledAt()).to.be.at.least(now);

        expect(identity.getPublicKeyById(0)).to.exist();
        expect(identity.getPublicKeyById(0).getDisabledAt()).to.be.undefined();
      });
    });

    describe('Masternodes', () => {
      it('should receive masternode identities', async () => {
        await client.platform.initialize();

        const { mnList } = await client.getDAPIClient().dapiAddressProvider
          .smlProvider.getSimplifiedMNList();

        for (const masternodeEntry of mnList) {
          if (!masternodeEntry.isValid) {
            // eslint-disable-next-line no-continue
            continue;
          }

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

            const operatorIdentityHash = sha256(
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

          const publicKeyOwner = Buffer.from(transaction.extraPayload.keyIDOwner, 'hex').reverse();
          const votingPubKeyHash = Buffer.from(transaction.extraPayload.keyIDVoting, 'hex').reverse();

          if (!votingPubKeyHash.equals(publicKeyOwner)) {
            const votingIdentityId = Identifier.from(
              hash(
                Buffer.concat([
                  Buffer.from(masternodeEntry.proRegTxHash, 'hex'),
                  votingPubKeyHash,
                ]),
              ),
            );

            fetchedIdentity = await client.platform.identities.get(
              votingIdentityId,
            );

            expect(fetchedIdentity).to.be.not.null();
          }
        }
      });
    });
  });
});
