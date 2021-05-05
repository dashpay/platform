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

describe('commitHandlerFactory', () => {
  let commitHandler;
  let appHash;
  let chainInfoMock;
  let chainInfoRepositoryMock;
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

  beforeEach(function beforeEach() {
    nextPreviousBlockExecutionStoreTransactionsMock = 'nextPreviousBlockExecutionStoreTransactionsMock';
    appHash = Buffer.alloc(0);

    chainInfoMock = {
      setLastBlockHeight: this.sinon.stub(),
    };

    creditsDistributionPoolMock = {
      incrementAmount: this.sinon.stub(),
      setAmount: this.sinon.stub(),
    };

    dataContract = getDataContractFixture();

    chainInfoRepositoryMock = {
      store: this.sinon.stub(),
      createTransaction: this.sinon.stub(),
    };

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

    documentsDatabaseManagerMock = {
      create: this.sinon.stub(),
      drop: this.sinon.stub(),
    };

    rootTreeMock = new RootTreeMock(this.sinon);
    rootTreeMock.getRootHash.returns(appHash);

    dppMock = createDPPMock(this.sinon);
    dppMock.dataContract.createFromBuffer.resolves(dataContract);

    previousBlockExecutionStoreTransactionsRepositoryMock = {
      fetch: this.sinon.stub(),
      store: this.sinon.stub(),
    };

    containerMock = {
      register: this.sinon.stub(),
      resolve: this.sinon.stub(),
      has: this.sinon.stub(),
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

    commitHandler = commitHandlerFactory(
      chainInfoMock,
      chainInfoRepositoryMock,
      creditsDistributionPoolMock,
      creditsDistributionPoolRepositoryMock,
      blockExecutionStoreTransactionsMock,
      blockExecutionContextMock,
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
    );
  });

  it('should call setAmount instead of incrementAmount if feature flag was not set', async () => {
    containerMock.has.withArgs('previousBlockExecutionStoreTransactions').returns(false);

    getLatestFeatureFlagMock.resolves(null);

    await commitHandler();

    expect(creditsDistributionPoolMock.setAmount).to.be.calledOnceWith(
      accumulativeFees,
    );
  });

  it('should call setAmount instead of incrementAmount if feature flag was set to false', async () => {
    containerMock.has.withArgs('previousBlockExecutionStoreTransactions').returns(false);

    getLatestFeatureFlagMock.resolves({
      get: () => false,
    });

    await commitHandler();

    expect(creditsDistributionPoolMock.setAmount).to.be.calledOnceWith(
      accumulativeFees,
    );
  });

  it('should commit db transactions, update chain info, create document dbs and return ResponseCommit', async () => {
    containerMock.has.withArgs('previousBlockExecutionStoreTransactions').returns(false);

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

    expect(blockExecutionStoreTransactionsMock.getTransaction).to.be.calledTwice();
    expect(blockExecutionStoreTransactionsMock.getTransaction.getCall(0).args).to.deep.equal(['documents']);
    expect(blockExecutionStoreTransactionsMock.getTransaction.getCall(1).args).to.deep.equal(['common']);

    expect(chainInfoRepositoryMock.store).to.be.calledOnceWith(chainInfoMock);
    expect(creditsDistributionPoolRepositoryMock.store).to.be.calledOnceWith(
      creditsDistributionPoolMock,
    );

    expect(cloneToPreviousStoreTransactionsMock).to.be.calledOnce();

    expect(blockExecutionStoreTransactionsMock.commit).to.be.calledOnce();

    expect(rootTreeMock.rebuild).to.be.calledOnce();
    expect(rootTreeMock.getRootHash.callCount).to.equal(1);

    expect(previousBlockExecutionStoreTransactionsRepositoryMock.store).to.be.calledOnceWithExactly(
      nextPreviousBlockExecutionStoreTransactionsMock,
    );
  });

  it('should commit db transactions, update chain info, create document dbs and return ResponseCommit on height > 1', async () => {
    header.height = Long.fromInt(2);

    containerMock.has.withArgs('previousBlockExecutionStoreTransactions').returns(true);

    containerMock.resolve.withArgs('previousBlockExecutionStoreTransactions').returns(
      previousBlockExecutionStoreTransactionsMock,
    );

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('dataContracts').returns(
      previousDataContractTransactionMock,
    );

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('documents').returns(
      previousDataContractTransactionMock,
    );

    const response = await commitHandler();

    expect(response).to.be.an.instanceOf(ResponseCommit);
    expect(response.data).to.deep.equal(appHash);

    expect(blockExecutionContextMock.getHeader).to.be.calledOnce();

    expect(blockExecutionContextMock.getDataContracts).to.be.calledOnce();
    expect(documentsDatabaseManagerMock.create).to.be.calledOnceWithExactly(dataContract);
    expect(creditsDistributionPoolMock.incrementAmount).to.be.calledOnceWith(accumulativeFees);
    expect(blockExecutionContextMock.getCumulativeFees).to.be.calledOnce();

    expect(blockExecutionStoreTransactionsMock.getTransaction).to.be.calledTwice();
    expect(blockExecutionStoreTransactionsMock.getTransaction.getCall(0).args).to.deep.equal(['documents']);
    expect(blockExecutionStoreTransactionsMock.getTransaction.getCall(1).args).to.deep.equal(['common']);

    expect(chainInfoRepositoryMock.store).to.be.calledOnceWith(chainInfoMock);
    expect(creditsDistributionPoolRepositoryMock.store).to.be.calledOnceWith(
      creditsDistributionPoolMock,
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

    containerMock.has.withArgs('previousBlockExecutionStoreTransactionsMock').returns(true);

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('dataContracts').returns(
      previousDataContractTransactionMock,
    );

    previousBlockExecutionStoreTransactionsMock.getTransaction.withArgs('documents').returns(
      previousDataContractTransactionMock,
    );

    const error = new Error('commit error');

    blockExecutionStoreTransactionsMock.commit.throws(error);

    try {
      await commitHandler();

      expect.fail('should throw error');
    } catch (e) {
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

      expect(chainInfoMock.setLastBlockHeight).to.be.calledOnceWithExactly(Long.fromInt(0));
      expect(chainInfoRepositoryMock.store).to.be.calledTwice();
    }
  });
});
