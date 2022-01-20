const {
  tendermint: {
    abci: {
      ResponseCommit,
    },
  },
} = require('@dashevo/abci/types');

const Long = require('long');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const commitHandlerFactory = require('../../../../lib/abci/handlers/commitHandlerFactory');

const RootTreeMock = require('../../../../lib/test/mock/RootTreeMock');

const BlockExecutionDBTransactionsMock = require('../../../../lib/test/mock/BlockExecutionStoreTransactionsMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const DataCorruptedError = require('../../../../lib/abci/handlers/errors/DataCorruptedError');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');
const BlockExecutionContextRepository = require('../../../../lib/blockExecution/BlockExecutionContextRepository');

describe('commitHandlerFactory', () => {
  let commitHandler;
  let appHash;
  let creditsDistributionPoolMock;
  let creditsDistributionPoolRepositoryMock;
  let blockExecutionStoreTransactionsMock;
  let blockExecutionContextMock;
  let documentsDatabaseManagerMock;
  let dataContract;
  let accumulativeFees;
  let rootTreeMock;
  let dppMock;
  let previousBlockExecutionStoreTransactionsRepositoryMock;
  let containerMock;
  let previousDocumentDatabaseManagerMock;
  let nextPreviousBlockExecutionStoreTransactionsMock;
  let previousBlockExecutionStoreTransactionsMock;
  let previousDataContractTransactionMock;
  let updates;
  let populateMongoDbTransactionFromObjectMock;
  let mongoDbTransactionMock;
  let cloneToPreviousStoreTransactionsMock;
  let header;
  let getLatestFeatureFlagMock;
  let previousBlockExecutionContextMock;
  let blockExecutionContextRepositoryMock;
  let previousRootTreeMock;
  let dataContractCacheMock;

  beforeEach(function beforeEach() {
    nextPreviousBlockExecutionStoreTransactionsMock = 'nextPreviousBlockExecutionStoreTransactionsMock';
    appHash = Buffer.alloc(0);

    creditsDistributionPoolMock = {
      incrementAmount: this.sinon.stub(),
      setAmount: this.sinon.stub(),
    };

    dataContract = getDataContractFixture();

    blockExecutionStoreTransactionsMock = new BlockExecutionDBTransactionsMock(this.sinon);
    creditsDistributionPoolRepositoryMock = {
      store: this.sinon.stub(),
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getDataContracts.returns([dataContract]);
    blockExecutionContextMock.getCumulativeFees.returns(accumulativeFees);

    header = {
      height: Long.fromInt(1),
    };

    blockExecutionContextMock.getHeader.returns(header);

    previousBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextRepositoryMock = {
      fetch: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    documentsDatabaseManagerMock = {
      create: this.sinon.stub(),
      drop: this.sinon.stub(),
    };

    rootTreeMock = new RootTreeMock(this.sinon);
    rootTreeMock.getRootHash.returns(appHash);

    previousRootTreeMock = new RootTreeMock(this.sinon);

    dppMock = createDPPMock(this.sinon);
    dppMock.dataContract.createFromBuffer.resolves(dataContract);

    previousBlockExecutionStoreTransactionsRepositoryMock = {
      fetch: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    containerMock = {
      register: this.sinon.stub(),
      resolve: this.sinon.stub(),
      hasRegistration: this.sinon.stub(),
    };

    const loggerMock = new LoggerMock(this.sinon);

    previousBlockExecutionStoreTransactionsMock = {
      getTransaction: this.sinon.stub(),
      commit: this.sinon.stub(),
    };

    updates = {
      dataContract: dataContract.toBuffer(),
    };

    mongoDbTransactionMock = 'mongoDbTransactionMock';

    previousDataContractTransactionMock = {
      getMongoDbTransaction: this.sinon.stub().returns(mongoDbTransactionMock),
      toObject: this.sinon.stub().returns({
        updates,
      }),
    };

    previousDocumentDatabaseManagerMock = {
      create: this.sinon.stub(),
    };

    populateMongoDbTransactionFromObjectMock = this.sinon.stub();
    cloneToPreviousStoreTransactionsMock = this.sinon.stub();

    cloneToPreviousStoreTransactionsMock.returns(
      nextPreviousBlockExecutionStoreTransactionsMock,
    );

    getLatestFeatureFlagMock = this.sinon.stub();
    getLatestFeatureFlagMock.resolves({
      get: () => true,
    });

    dataContractCacheMock = {
      set: this.sinon.stub(),
      get: this.sinon.stub(),
      has: this.sinon.stub(),
    };

    commitHandler = commitHandlerFactory(
      creditsDistributionPoolMock,
      creditsDistributionPoolRepositoryMock,
      blockExecutionStoreTransactionsMock,
      blockExecutionContextMock,
      previousBlockExecutionContextMock,
      blockExecutionContextRepositoryMock,
      documentsDatabaseManagerMock,
      previousDocumentDatabaseManagerMock,
      dppMock,
      rootTreeMock,
      previousBlockExecutionStoreTransactionsRepositoryMock,
      populateMongoDbTransactionFromObjectMock,
      containerMock,
      loggerMock,
      cloneToPreviousStoreTransactionsMock,
      getLatestFeatureFlagMock,
      previousRootTreeMock,
      dataContractCacheMock,
    );
  });

  describe('Cumulative fees', () => {
    it('should call setAmount instead of incrementAmount if feature flag was not set', async () => {
      containerMock.hasRegistration.withArgs('previousBlockExecutionStoreTransactions').returns(false);

      getLatestFeatureFlagMock.resolves(null);

      await commitHandler();

      expect(creditsDistributionPoolMock.incrementAmount).to.be.calledOnceWith(
        accumulativeFees,
      );
    });

    it('should call setAmount instead of incrementAmount if feature flag was set to false', async () => {
      containerMock.hasRegistration.withArgs('previousBlockExecutionStoreTransactions').returns(false);

      getLatestFeatureFlagMock.resolves({
        get: () => false,
      });

      await commitHandler();

      expect(creditsDistributionPoolMock.incrementAmount).to.be.calledOnceWith(
        accumulativeFees,
      );
    });
  });

  it('should commit db transactions, create document dbs and return ResponseCommit', async () => {
    containerMock.hasRegistration.withArgs('previousBlockExecutionStoreTransactions').returns(false);

    previousBlockExecutionContextMock.isEmpty.returns(true);

    const response = await commitHandler();

    expect(response).to.be.an.instanceOf(ResponseCommit);
    expect(response.data).to.deep.equal(appHash);

    expect(blockExecutionContextMock.getHeader).to.be.calledOnce();

    expect(blockExecutionContextMock.getDataContracts).to.be.calledOnce();

    expect(documentsDatabaseManagerMock.create).to.be.calledOnceWith(dataContract);

    expect(blockExecutionStoreTransactionsMock.commit).to.be.calledOnce();

    expect(creditsDistributionPoolMock.incrementAmount).to.be.calledOnceWith(
      accumulativeFees,
    );

    expect(blockExecutionContextMock.getCumulativeFees).to.be.calledOnce();

    expect(blockExecutionStoreTransactionsMock.getTransaction).to.be.calledOnce();
    expect(blockExecutionStoreTransactionsMock.getTransaction.getCall(0).args).to.deep.equal(['common']);

    expect(blockExecutionContextRepositoryMock.store).to.be.calledOnceWithExactly(
      BlockExecutionContextRepository.KEY_PREFIX_CURRENT,
      blockExecutionContextMock,
    );

    expect(cloneToPreviousStoreTransactionsMock).to.be.calledOnce();

    expect(blockExecutionStoreTransactionsMock.commit).to.be.calledOnce();

    expect(rootTreeMock.rebuild).to.be.calledOnce();
    expect(rootTreeMock.getRootHash.callCount).to.equal(1);

    expect(previousBlockExecutionStoreTransactionsRepositoryMock.store).to.be.calledOnceWithExactly(
      nextPreviousBlockExecutionStoreTransactionsMock,
    );
  });

  it('should commit db transactions, create document dbs and return ResponseCommit on height > 1', async () => {
    header.height = Long.fromInt(2);

    containerMock.hasRegistration.withArgs('previousBlockExecutionStoreTransactions').returns(true);

    containerMock.resolve.withArgs('previousBlockExecutionStoreTransactions').returns(
      previousBlockExecutionStoreTransactionsMock,
    );

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('dataContracts').returns(
      previousDataContractTransactionMock,
    );

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('documents').returns(
      previousDataContractTransactionMock,
    );

    previousBlockExecutionContextMock.isEmpty.returns(false);

    const response = await commitHandler();

    expect(response).to.be.an.instanceOf(ResponseCommit);
    expect(response.data).to.deep.equal(appHash);

    expect(blockExecutionContextMock.getHeader).to.be.calledOnce();

    expect(blockExecutionContextMock.getDataContracts).to.be.calledOnce();
    expect(documentsDatabaseManagerMock.create).to.be.calledOnceWithExactly(dataContract);
    expect(creditsDistributionPoolMock.incrementAmount).to.be.calledOnceWith(accumulativeFees);
    expect(blockExecutionContextMock.getCumulativeFees).to.be.calledOnce();

    expect(blockExecutionStoreTransactionsMock.getTransaction).to.be.calledOnce();
    expect(blockExecutionStoreTransactionsMock.getTransaction.getCall(0).args).to.deep.equal(['common']);

    expect(blockExecutionContextRepositoryMock.store).to.be.calledTwice();
    expect(blockExecutionContextRepositoryMock.store.getCall(0)).to.be.calledWithExactly(
      BlockExecutionContextRepository.KEY_PREFIX_CURRENT,
      blockExecutionContextMock,
    );
    expect(blockExecutionContextRepositoryMock.store.getCall(1)).to.be.calledWithExactly(
      BlockExecutionContextRepository.KEY_PREFIX_PREVIOUS,
      previousBlockExecutionContextMock,
    );

    expect(cloneToPreviousStoreTransactionsMock).to.be.calledOnce();
    expect(blockExecutionStoreTransactionsMock.commit).to.be.calledOnce();

    expect(previousBlockExecutionStoreTransactionsMock.getTransaction).to.be.calledTwice();
    expect(previousBlockExecutionStoreTransactionsMock.getTransaction.getCall(0).args).to.have.deep.members(['dataContracts']);
    expect(previousBlockExecutionStoreTransactionsMock.getTransaction.getCall(1).args).to.have.deep.members(['documents']);

    expect(previousDataContractTransactionMock.toObject).to.be.calledTwice();

    expect(previousDocumentDatabaseManagerMock.create).to.be.calledOnceWithExactly(dataContract);

    expect(populateMongoDbTransactionFromObjectMock).to.be.calledOnce();
    expect(populateMongoDbTransactionFromObjectMock.getCall(0).args).to.have.deep.members([
      mongoDbTransactionMock,
      { updates },
    ]);

    expect(previousBlockExecutionStoreTransactionsMock.commit).to.be.calledOnce();

    expect(rootTreeMock.rebuild).to.be.calledOnce();
    expect(rootTreeMock.getRootHash.callCount).to.equal(1);

    expect(previousBlockExecutionStoreTransactionsRepositoryMock.store).to.be.calledOnceWithExactly(
      nextPreviousBlockExecutionStoreTransactionsMock,
    );
  });

  it('should abort DB transactions', async () => {
    header.height = Long.fromInt(2);

    containerMock.hasRegistration.withArgs('previousBlockExecutionStoreTransactionsMock').returns(true);

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('dataContracts').returns(
      previousDataContractTransactionMock,
    );

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('documents').returns(
      previousDataContractTransactionMock,
    );

    blockExecutionStoreTransactionsMock.isStarted.returns(true);

    const error = new Error('commit error');

    blockExecutionStoreTransactionsMock.commit.throws(error);

    try {
      await commitHandler();

      expect.fail('should throw error');
    } catch (e) {
      expect(blockExecutionStoreTransactionsMock.isStarted).to.be.calledOnceWithExactly();
      expect(blockExecutionStoreTransactionsMock.abort).to.be.calledOnce();
      expect(documentsDatabaseManagerMock.drop).to.be.calledOnce();

      expect(e).to.deep.equal(error);
    }
  });

  it('should throw DataCorruptedError', async () => {
    const error = new Error('store error');

    previousBlockExecutionStoreTransactionsRepositoryMock.store.throws(error);

    try {
      await commitHandler();

      expect.fail('should throw DataCorruptedError');
    } catch (e) {
      expect(e).to.be.an.instanceOf(DataCorruptedError);
    }
  });
});
