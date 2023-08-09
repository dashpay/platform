const { ReadOperation, StateTransitionExecutionContext } = require('@dashevo/wasm-dpp');
const getDocumentsFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDocumentsFixture');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');
const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');
const generateRandomIdentifier = require('@dashevo/wasm-dpp/lib/test/utils/generateRandomIdentifierAsync');
const getInstantLockFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getInstantLockFixture');

const Long = require('long');

const QuorumEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/QuorumEntry');
const DriveStateRepository = require('../../../lib/dpp/DriveStateRepository');
const StorageResult = require('../../../lib/storage/StorageResult');
const BlockExecutionContextMock = require('../../../lib/test/mock/BlockExecutionContextMock');
const BlockInfo = require('../../../lib/blockExecution/BlockInfo');

describe('DriveStateRepository', () => {
  let stateRepository;
  let identityRepositoryMock;
  let identityBalanceRepositoryMock;
  let identityPublicKeyRepositoryMock;
  let dataContractRepositoryMock;
  let fetchDocumentsMock;
  let documentsRepositoryMock;
  let spentAssetLockTransactionsRepositoryMock;
  let coreRpcClientMock;
  let id;
  let identity;
  let documents;
  let dataContract;
  let blockExecutionContextMock;
  let simplifiedMasternodeListMock;
  let instantLockFixture;
  let repositoryOptions;
  let executionContext;
  let operations;
  let blockInfo;
  let rsDriveMock;
  let blockHeight;
  let timeMs;

  beforeEach(async function beforeEach() {
    identity = await getIdentityFixture();
    documents = await getDocumentsFixture();
    dataContract = await getDataContractFixture();
    id = await generateRandomIdentifier();

    coreRpcClientMock = {
      getRawTransaction: this.sinon.stub(),
      verifyIsLock: this.sinon.stub(),
      quorum: this.sinon.stub(),
    };

    dataContractRepositoryMock = {
      fetch: this.sinon.stub(),
      create: this.sinon.stub(),
      update: this.sinon.stub(),
    };

    identityRepositoryMock = {
      fetch: this.sinon.stub(),
      create: this.sinon.stub(),
      updateRevision: this.sinon.stub(),
    };

    identityBalanceRepositoryMock = {
      add: this.sinon.stub(),
      fetch: this.sinon.stub(),
      fetchWithDebt: this.sinon.stub(),
    };

    identityPublicKeyRepositoryMock = {
      fetch: this.sinon.stub(),
      add: this.sinon.stub(),
      disable: this.sinon.stub(),
    };

    fetchDocumentsMock = this.sinon.stub();

    documentsRepositoryMock = {
      create: this.sinon.stub(),
      update: this.sinon.stub(),
      find: this.sinon.stub(),
      delete: this.sinon.stub(),
    };

    spentAssetLockTransactionsRepositoryMock = {
      store: this.sinon.stub(),
      find: this.sinon.stub(),
      delete: this.sinon.stub(),
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    timeMs = Date.now();

    blockHeight = Long.fromNumber(1);

    blockInfo = new BlockInfo(blockHeight.toNumber(), 0, timeMs);

    blockExecutionContextMock.getEpochInfo.returns({
      currentEpochIndex: blockInfo.epoch,
    });
    blockExecutionContextMock.getHeight.returns(blockHeight);
    blockExecutionContextMock.getTimeMs.returns(timeMs);

    simplifiedMasternodeListMock = {
      getStore: this.sinon.stub(),
    };

    repositoryOptions = { useTransaction: true };

    rsDriveMock = {
      fetchLatestWithdrawalTransactionIndex: this.sinon.stub(),
      enqueueWithdrawalTransaction: this.sinon.stub(),
    };

    rsDriveMock.fetchLatestWithdrawalTransactionIndex.resolves(42);

    stateRepository = new DriveStateRepository(
      identityRepositoryMock,
      identityBalanceRepositoryMock,
      identityPublicKeyRepositoryMock,
      dataContractRepositoryMock,
      fetchDocumentsMock,
      documentsRepositoryMock,
      spentAssetLockTransactionsRepositoryMock,
      coreRpcClientMock,
      blockExecutionContextMock,
      simplifiedMasternodeListMock,
      rsDriveMock,
      repositoryOptions,
    );

    instantLockFixture = getInstantLockFixture();

    instantLockFixture.selectSignatoryRotatedQuorum = this.sinon.stub();

    executionContext = new StateTransitionExecutionContext();
    operations = [new ReadOperation(1)];
  });

  describe('#fetchIdentity', () => {
    it('should fetch identity from repository', async () => {
      identityRepositoryMock.fetch.resolves(
        new StorageResult(identity, operations),
      );

      const result = await stateRepository.fetchIdentity(id, executionContext);

      expect(result).to.equal(identity);
      expect(identityRepositoryMock.fetch).to.be.calledOnceWith(
        id,
        {
          blockInfo,
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations().length).to.be.equal(1);
      expect(
        executionContext.getOperations()[0].toJSON(),
      ).to.deep.equals(
        operations[0].toJSON(),
      );
    });
  });

  describe('#createIdentity', () => {
    it('should create identity', async () => {
      identityRepositoryMock.create.resolves(
        new StorageResult(undefined, operations),
      );

      await stateRepository.createIdentity(identity, executionContext);

      expect(identityRepositoryMock.create).to.be.calledOnceWith(
        identity,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#fetchIdentityBalance', () => {
    it('should fetch identity balance', async () => {
      identityBalanceRepositoryMock.fetch.resolves(
        new StorageResult(1, operations),
      );

      const result = await stateRepository.fetchIdentityBalance(identity.getId(), executionContext);

      expect(result).to.equals(1);

      expect(identityBalanceRepositoryMock.fetch).to.be.calledOnceWith(
        identity.getId(),
        {
          blockInfo,
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#fetchIdentityBalanceWithDebt', () => {
    it('should fetch identity balance', async () => {
      identityBalanceRepositoryMock.fetchWithDebt.resolves(
        new StorageResult(1, operations),
      );

      const result = await stateRepository.fetchIdentityBalanceWithDebt(
        identity.getId(),
        executionContext,
      );

      expect(result).to.equals(1);

      expect(identityBalanceRepositoryMock.fetchWithDebt).to.be.calledOnceWith(
        identity.getId(),
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#addToIdentityBalance', () => {
    it('should update identity balance', async () => {
      identityBalanceRepositoryMock.add.resolves(
        new StorageResult(undefined, operations),
      );

      await stateRepository.addToIdentityBalance(identity.getId(), 1, executionContext);

      expect(identityBalanceRepositoryMock.add).to.be.calledOnceWith(
        identity.getId(),
        1,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#updateIdentityRevision', () => {
    it('should update identity revision', async () => {
      identityRepositoryMock.updateRevision.resolves(
        new StorageResult(undefined, operations),
      );

      await stateRepository.updateIdentityRevision(identity.getId(), 1, executionContext);

      expect(identityRepositoryMock.updateRevision).to.be.calledOnceWith(
        identity.getId(),
        1,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#addKeysToIdentity', () => {
    it('should add keys to identity', async () => {
      identityPublicKeyRepositoryMock.add.resolves(
        new StorageResult(undefined, operations),
      );

      await stateRepository.addKeysToIdentity(
        identity.getId(),
        identity.getPublicKeys(),
        executionContext,
      );

      expect(identityPublicKeyRepositoryMock.add).to.be.calledOnceWith(
        identity.getId(),
        identity.getPublicKeys().map((key) => key.toObject()),
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#disableIdentityKeys', () => {
    it('should disable identity keys', async () => {
      identityPublicKeyRepositoryMock.disable.resolves(
        new StorageResult(undefined, operations),
      );

      await stateRepository.disableIdentityKeys(
        identity.getId(),
        [1, 2],
        123,
        executionContext,
      );

      expect(identityPublicKeyRepositoryMock.disable).to.be.calledOnceWith(
        identity.getId(),
        [1, 2],
        123,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#fetchDataContract', () => {
    it('should fetch data contract from repository', async () => {
      dataContractRepositoryMock.fetch.resolves(
        new StorageResult(dataContract, operations),
      );

      const result = await stateRepository.fetchDataContract(id, executionContext);

      expect(result).to.equal(dataContract);
      expect(dataContractRepositoryMock.fetch).to.be.calledOnceWithExactly(
        id,
        {
          blockInfo,
          dryRun: false,
          useTransaction: repositoryOptions.useTransaction,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#createDataContract', () => {
    it('should create data contract to repository', async () => {
      dataContractRepositoryMock.create.resolves(
        new StorageResult(undefined, operations),
      );

      await stateRepository.createDataContract(dataContract, executionContext);

      expect(dataContractRepositoryMock.create).to.be.calledOnceWith(
        dataContract,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#updateDataContract', () => {
    it('should store data contract to repository', async () => {
      dataContractRepositoryMock.update.resolves(
        new StorageResult(undefined, operations),
      );

      await stateRepository.updateDataContract(dataContract, executionContext);

      expect(dataContractRepositoryMock.update).to.be.calledOnceWith(
        dataContract,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#fetchDocuments', () => {
    it('should fetch documents from repository', async () => {
      const type = 'documentType';
      const options = {};

      fetchDocumentsMock.resolves(
        new StorageResult(documents, operations),
      );

      const result = await stateRepository.fetchDocuments(
        id,
        type,
        options,
        executionContext,
      );

      expect(result).to.equal(documents);
      expect(fetchDocumentsMock).to.be.calledOnceWith(
        id,
        type,
        {
          blockInfo,
          ...options,
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#createDocument', () => {
    it('should create document in repository', async () => {
      documentsRepositoryMock.create.resolves(
        new StorageResult(undefined, operations),
      );

      const [document] = documents;

      await stateRepository.createDocument(document, executionContext);

      expect(documentsRepositoryMock.create).to.be.calledOnceWith(
        document,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#updateDocument', () => {
    it('should store document in repository', async () => {
      documentsRepositoryMock.update.resolves(
        new StorageResult(undefined, operations),
      );

      const [document] = documents;

      await stateRepository.updateDocument(document, executionContext);

      expect(documentsRepositoryMock.update).to.be.calledOnceWith(
        document,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#removeDocument', () => {
    it('should delete document from repository', async () => {
      documentsRepositoryMock.delete.resolves(
        new StorageResult(undefined, operations),
      );

      const type = 'documentType';

      await stateRepository.removeDocument(dataContract, type, id, executionContext);

      expect(documentsRepositoryMock.delete).to.be.calledOnceWith(
        dataContract,
        type,
        id,
        blockInfo,
        {
          useTransaction: repositoryOptions.useTransaction,
          dryRun: false,
        },
      );

      expect(executionContext.getOperations()).to.deep.equals(operations);
    });
  });

  describe('#fetchTransaction', () => {
    it('should fetch transaction from core', async () => {
      const rawTransaction = {
        hex: 'some result',
        height: 1,
      };

      coreRpcClientMock.getRawTransaction.resolves({ result: rawTransaction });

      const result = await stateRepository.fetchTransaction(id, executionContext);

      expect(result).to.deep.equal({
        data: Buffer.from(rawTransaction.hex, 'hex'),
        height: rawTransaction.height,
      });

      expect(coreRpcClientMock.getRawTransaction).to.be.calledOnceWithExactly(id, 1);

      const operation = new ReadOperation(Buffer.from(rawTransaction.hex, 'hex').length);

      expect(executionContext.getOperations()).to.deep.equals([operation]);
    });

    it('should return null if core throws Invalid address or key error', async () => {
      const error = new Error('Some error');
      error.code = -5;

      coreRpcClientMock.getRawTransaction.throws(error);

      const result = await stateRepository.fetchTransaction(id);

      expect(result).to.equal(null);
      expect(coreRpcClientMock.getRawTransaction).to.be.calledOnceWith(id);
    });

    it('should throw an error if core throws an unknown error', async () => {
      const error = new Error('Some error');

      coreRpcClientMock.getRawTransaction.throws(error);

      try {
        await stateRepository.fetchTransaction(id);

        expect.fail('should throw error');
      } catch (e) {
        expect(e).to.equal(error);
        expect(coreRpcClientMock.getRawTransaction).to.be.calledOnceWith(id);
      }
    });

    it('should return mocked transaction on dry run', async () => {
      executionContext.enableDryRun();

      const result = await stateRepository.fetchTransaction(id, executionContext);

      executionContext.disableDryRun();

      expect(result).to.deep.equal({
        data: Buffer.alloc(0),
        height: 1,
      });

      expect(coreRpcClientMock.getRawTransaction).to.not.be.called(id);
    });
  });

  describe('#fetchLatestPlatformBlockHeight', () => {
    it('should fetch latest platform block height', async () => {
      blockExecutionContextMock.getHeight.resolves(10);

      const result = await stateRepository.fetchLatestPlatformBlockHeight();

      expect(result).to.equal(10);
      expect(blockExecutionContextMock.getHeight).to.be.calledOnce();
    });
  });

  describe('#fetchLatestPlatformBlockTime', () => {
    it('should fetch latest platform block time', async () => {
      const result = await stateRepository.fetchLatestPlatformBlockTime();

      expect(result).to.deep.equal(timeMs);
      expect(blockExecutionContextMock.getTimeMs).to.be.calledOnce();
    });
  });

  describe('#fetchLatestPlatformCoreChainLockedHeight', () => {
    it('should fetch latest platform core chainlocked height', async () => {
      blockExecutionContextMock.getCoreChainLockedHeight.returns(10);

      const result = await stateRepository.fetchLatestPlatformCoreChainLockedHeight();

      expect(result).to.equal(10);
      expect(blockExecutionContextMock.getCoreChainLockedHeight).to.be.calledOnce();
    });
  });

  describe('#verifyInstantLock', () => {
    let smlStoreMock;
    let instantlockSMLMock;
    let llmqType;

    beforeEach(function beforeEach() {
      llmqType = 103;
      blockExecutionContextMock.getHeight.returns(41);
      blockExecutionContextMock.getCoreChainLockedHeight.returns(42);

      instantlockSMLMock = {
        getInstantSendLLMQType: this.sinon.stub(),
        isLLMQTypeRotated: this.sinon.stub(),
        getQuorumsOfType: this.sinon.stub(),
      };

      instantlockSMLMock.getInstantSendLLMQType.returns(llmqType);
      instantlockSMLMock.isLLMQTypeRotated.returns(false);
      instantlockSMLMock.getQuorumsOfType.returns([
        new QuorumEntry({
          version: 4,
          llmqType: 105,
          quorumHash: '0000059f6b83762d1801d74e9ece78790a4fedabc79e69f614957dde04b2c3dd',
          signersCount: 8,
          signers: 'ff',
          validMembersCount: 8,
          validMembers: 'ff',
          quorumPublicKey: 'b9f86173367775b411340f1c911bf5478ae10feb11029700e645f12fa6fad54c615783f3c3a742b792c8de6853a30137',
          quorumVvecHash: 'd1caaf60458073036a7616ec85e16a21fb16f79ccf764ad7eb26650d082aac2f',
          quorumSig: 'ad378a9df7711b5439c04b5b726d635427d6e3fbb1b37f31c06adafabaabd94007442dfad3c1f7da9c7221ddca9ae1bc096aa8dff900d477add6a4aafd2ad1afe4ba702f37ad1415cab513e9775e3fd1f5398a807e64a5a70b9a408140b92f2c',
          membersSig: '90a7a0e92d860dfd1644182b819bcb2edfcc6ff0331b70cb1baab9de8f760aa39f2cd1c878ea7547de79dd95702b00db0495675ed6d2538d33c659d308566866a017ea5c58203e396433b681e3d636afb6a11b25dd5f49111160354fe759f3c6',
          quorumIndex: 1,
        }),
        new QuorumEntry({
          version: 4,
          llmqType: 105,
          quorumHash: '00000084db75bc85dc66d6fd7f569283415b06afc4a5d33e746b96470339359b',
          signersCount: 8,
          signers: 'ff',
          validMembersCount: 8,
          validMembers: 'ff',
          quorumPublicKey: 'a67f842d6130f0cafa6c035ab8c0d53dcf1fb78dd01f40a93db709b68db761c6a88912b3408ed1a63cdf1020fdb285a4',
          quorumVvecHash: '5a9f2daa1f2f833fbf3b03bb0e2b12b5313a3fb917ba369ff84c2e23849cdb2a',
          quorumSig: 'a075da563cac4013fbc95d3231dd2bc3563bf6c43dabd4f066ec5846b28f1694dbc602dc417b34aa8d97e52c11c4348002c7793133404f1792aa8eb3e4c5811d57bfd8fb46667dd559be2f4a4459854ab4aa2b8741534edf247a3d671a8bc68c',
          membersSig: 'b6a4c84a1d182f1f3c66b680f4cc4e15bffaa5c71d9da12284054f22661bded5231d7294f5d24ddbc7908e647636ecbd0492f366560fd29bfb7fa91908902099e223e7f754cb2d9f77c388fb4150f812c9dcbcc688c084ad05ecd3f80ce7b9b5',
          quorumIndex: 0,
        }),
      ]);

      smlStoreMock = {
        getSMLbyHeight: this.sinon.stub(),
        getTipHeight: this.sinon.stub(),
      };

      smlStoreMock.getSMLbyHeight.returns(instantlockSMLMock);

      simplifiedMasternodeListMock.getStore.returns(smlStoreMock);
    });

    it('it should verify instant lock using Core', async () => {
      coreRpcClientMock.verifyIsLock.resolves({ result: true });

      const result = await stateRepository.verifyInstantLock(instantLockFixture.toBuffer());

      expect(result).to.equal(true);
      expect(coreRpcClientMock.verifyIsLock).to.have.been.calledOnceWithExactly(
        instantLockFixture.getRequestId().toString('hex'),
        instantLockFixture.txid,
        instantLockFixture.signature,
        42,
      );
      expect(coreRpcClientMock.quorum).to.have.not.been.called();
    });

    it('should return false if core throws Invalid address or key error', async () => {
      const error = new Error('Some error');
      error.code = -5;

      coreRpcClientMock.verifyIsLock.throws(error);

      const result = await stateRepository.verifyInstantLock(instantLockFixture.toBuffer());

      expect(result).to.equal(false);
      expect(coreRpcClientMock.verifyIsLock).to.have.been.calledOnceWithExactly(
        instantLockFixture.getRequestId().toString('hex'),
        instantLockFixture.txid,
        instantLockFixture.signature,
        42,
      );
    });

    it('should return false if core throws Invalid parameter', async () => {
      const error = new Error('Some error');
      error.code = -8;

      coreRpcClientMock.verifyIsLock.throws(error);

      const result = await stateRepository.verifyInstantLock(instantLockFixture.toBuffer());

      expect(result).to.equal(false);
      expect(coreRpcClientMock.verifyIsLock).to.have.been.calledOnceWithExactly(
        instantLockFixture.getRequestId().toString('hex'),
        instantLockFixture.txid,
        instantLockFixture.signature,
        42,
      );
    });

    it('should return false if coreChainLockedHeight is null', async () => {
      blockExecutionContextMock.getCoreChainLockedHeight.returns(null);

      const result = await stateRepository.verifyInstantLock(instantLockFixture.toBuffer());

      expect(result).to.be.false();
    });

    it('should return true on dry run', async () => {
      executionContext.enableDryRun();

      const result = await stateRepository.verifyInstantLock(instantLockFixture, executionContext);

      executionContext.disableDryRun();

      expect(result).to.be.true();
      expect(coreRpcClientMock.verifyIsLock).to.have.not.been.called();
    });

    it('should validate quorum using core', async () => {
      smlStoreMock.getTipHeight.returns(100);
      instantlockSMLMock.isLLMQTypeRotated.returns(true);
      coreRpcClientMock.verifyIsLock.resolves({ result: true });
      coreRpcClientMock.quorum.resolves({ result: { previousConsecutiveDKGFailures: 0 } });

      const result = await stateRepository.verifyInstantLock(instantLockFixture.toBuffer());

      expect(result).to.equal(true);
      expect(coreRpcClientMock.verifyIsLock).to.have.been.calledOnceWithExactly(
        'bbbb1cfeb55396d7e5f9bebdb220670d23dbb0b47e22b1cd5357afe1ef33f559',
        instantLockFixture.txid,
        '8967c46529a967b3822e1ba8a173066296d02593f0f59b3a78a30a7eef9c8a120847729e62e4a32954339286b79fe7590221331cd28d576887a263f45b595d499272f656c3f5176987c976239cac16f972d796ad82931d532102a4f95eec7d80',
        42,
      );
      expect(coreRpcClientMock.quorum).to.have.been.calledOnceWithExactly('info', llmqType, '0000059f6b83762d1801d74e9ece78790a4fedabc79e69f614957dde04b2c3dd');

      expect(simplifiedMasternodeListMock.getStore).to.have.been.calledOnce();
      expect(smlStoreMock.getSMLbyHeight).to.have.been.calledTwice();
      expect(smlStoreMock.getSMLbyHeight.getCall(0)).to.have.been.calledWithExactly(93);
      expect(smlStoreMock.getSMLbyHeight.getCall(1)).to.have.been.calledWithExactly(93);
    });

    it('should return false if previousConsecutiveDKGFailures > 0', async () => {
      smlStoreMock.getTipHeight.returns(100);
      instantlockSMLMock.isLLMQTypeRotated.returns(true);
      coreRpcClientMock.verifyIsLock.resolves({ result: true });
      coreRpcClientMock.quorum.resolves({ result: { previousConsecutiveDKGFailures: 1 } });

      const result = await stateRepository.verifyInstantLock(instantLockFixture.toBuffer());

      expect(result).to.equal(false);
      expect(coreRpcClientMock.verifyIsLock).to.have.not.been.called();
      expect(coreRpcClientMock.quorum).to.have.been.calledOnceWithExactly('info', llmqType, '0000059f6b83762d1801d74e9ece78790a4fedabc79e69f614957dde04b2c3dd');

      expect(simplifiedMasternodeListMock.getStore).to.have.been.calledOnce();
      expect(smlStoreMock.getSMLbyHeight).to.have.been.calledTwice();
      expect(smlStoreMock.getSMLbyHeight.getCall(0)).to.have.been.calledWithExactly(93);
      expect(smlStoreMock.getSMLbyHeight.getCall(1)).to.have.been.calledWithExactly(93);
    });
  });

  describe('#fetchSMLStore', () => {
    it('should fetch SML store', async () => {
      simplifiedMasternodeListMock.getStore.resolves('store');

      const result = await stateRepository.fetchSMLStore();

      expect(result).to.equal('store');
      expect(simplifiedMasternodeListMock.getStore).to.be.calledOnce();
    });
  });

  describe('#fetchLatestWithdrawalTransactionIndex', () => {
    it('should call fetchLatestWithdrawalTransactionIndex', async () => {
      const result = await stateRepository.fetchLatestWithdrawalTransactionIndex();

      expect(result).to.equal(42);
      expect(
        rsDriveMock.fetchLatestWithdrawalTransactionIndex,
      ).to.have.been.calledOnceWithExactly(
        blockInfo,
        repositoryOptions.useTransaction,
        repositoryOptions.dryRun,
      );
    });
  });

  describe('#enqueueWithdrawalTransaction', () => {
    it('should call enqueueWithdrawalTransaction', async () => {
      const index = 42;
      const transactionBytes = Buffer.alloc(32, 1);

      await stateRepository.enqueueWithdrawalTransaction(
        index, transactionBytes,
      );

      expect(
        rsDriveMock.enqueueWithdrawalTransaction,
      ).to.have.been.calledOnceWithExactly(
        index,
        transactionBytes,
        blockInfo,
        repositoryOptions.useTransaction,
      );
    });
  });
});
