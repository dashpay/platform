const {
  abci: {
    ResponseCommit,
  },
} = require('abci/types');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const commitHandlerFactory = require('../../../../lib/abci/handlers/commitHandlerFactory');

const RootTreeMock = require('../../../../lib/test/mock/RootTreeMock');

const BlockExecutionDBTransactionsMock = require('../../../../lib/test/mock/BlockExecutionDBTransactionsMock');
const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');

describe('commitHandlerFactory', () => {
  let commitHandler;
  let appHash;
  let chainInfoMock;
  let chainInfoRepositoryMock;
  let creditsDistributionPoolMock;
  let creditsDistributionPoolRepositoryMock;
  let blockExecutionDBTransactionsMock;
  let blockExecutionContextMock;
  let documentsDatabaseManagerMock;
  let dataContract;
  let accumulativeFees;
  let rootTreeMock;

  beforeEach(function beforeEach() {
    appHash = Buffer.alloc(0);

    chainInfoMock = {
      setLastBlockHeight: this.sinon.stub(),
    };

    creditsDistributionPoolMock = {
      setAmount: this.sinon.stub(),
    };

    dataContract = getDataContractFixture();

    chainInfoRepositoryMock = {
      store: this.sinon.stub(),
    };

    creditsDistributionPoolRepositoryMock = {
      store: this.sinon.stub(),
    };

    blockExecutionDBTransactionsMock = new BlockExecutionDBTransactionsMock(this.sinon);
    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getDataContracts.returns([dataContract]);
    blockExecutionContextMock.getAccumulativeFees.returns(accumulativeFees);
    blockExecutionContextMock.getHeader.returns({
      height: 42,
    });

    documentsDatabaseManagerMock = {
      create: this.sinon.stub(),
      drop: this.sinon.stub(),
    };

    rootTreeMock = new RootTreeMock(this.sinon);
    rootTreeMock.getRootHash.returns(appHash);

    const loggerMock = {
      debug: this.sinon.stub(),
      info: this.sinon.stub(),
    };

    commitHandler = commitHandlerFactory(
      chainInfoMock,
      chainInfoRepositoryMock,
      creditsDistributionPoolMock,
      creditsDistributionPoolRepositoryMock,
      blockExecutionDBTransactionsMock,
      blockExecutionContextMock,
      documentsDatabaseManagerMock,
      rootTreeMock,
      loggerMock,
    );
  });

  it('should commit db transactions, update chain info, create document dbs and return ResponseCommit', async () => {
    const response = await commitHandler();

    expect(response).to.be.an.instanceOf(ResponseCommit);
    expect(response.data).to.deep.equal(appHash);

    expect(blockExecutionContextMock.getDataContracts).to.be.calledOnce();

    expect(documentsDatabaseManagerMock.create).to.be.calledOnceWith(dataContract);

    expect(blockExecutionDBTransactionsMock.commit).to.be.calledOnce();

    expect(creditsDistributionPoolMock.setAmount).to.be.calledOnceWith(
      accumulativeFees,
    );

    expect(blockExecutionContextMock.getAccumulativeFees).to.be.calledOnce();

    expect(chainInfoRepositoryMock.store).to.be.calledOnceWith(chainInfoMock);
    expect(creditsDistributionPoolRepositoryMock.store).to.be.calledOnceWith(
      creditsDistributionPoolMock,
    );

    expect(rootTreeMock.rebuild).to.be.calledOnce();

    expect(blockExecutionContextMock.reset).to.be.calledOnce();

    expect(rootTreeMock.getRootHash).to.be.calledOnce();
  });

  it('should throw error and abort DB transactions if can\'t store chain info', async () => {
    const error = new Error('Some error');

    chainInfoRepositoryMock.store.throws(error);

    try {
      await commitHandler();

      expect.fail('should throw error');
    } catch (e) {
      expect(e).to.equal(error);

      expect(blockExecutionDBTransactionsMock.abort).to.be.calledOnce();
      expect(documentsDatabaseManagerMock.drop).to.be.calledOnceWith(dataContract);
      expect(blockExecutionContextMock.reset).to.be.calledOnce();
    }
  });
});
