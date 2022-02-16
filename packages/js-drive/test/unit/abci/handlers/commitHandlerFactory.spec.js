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

const BlockExecutionContextMock = require('../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../lib/test/mock/LoggerMock');

describe('commitHandlerFactory', () => {
  let commitHandler;
  let appHash;
  let creditsDistributionPoolMock;
  let creditsDistributionPoolRepositoryMock;
  let blockExecutionContextMock;
  let dataContract;
  let accumulativeFees;
  let rootTreeMock;
  let dppMock;
  let header;
  let dataContractCacheMock;
  let blockExecutionContextStackMock;
  let blockExecutionContextStackRepositoryMock;
  let groveDBStoreMock;
  let rotateSignedStoreMock;

  beforeEach(function beforeEach() {
    nextPreviousBlockExecutionStoreTransactionsMock = 'nextPreviousBlockExecutionStoreTransactionsMock';
    appHash = Buffer.alloc(0);

    creditsDistributionPoolMock = {
      incrementAmount: this.sinon.stub(),
      setAmount: this.sinon.stub(),
    };

    dataContract = getDataContractFixture();

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

    rootTreeMock = new RootTreeMock(this.sinon);
    rootTreeMock.getRootHash.returns(appHash);

    dppMock = createDPPMock(this.sinon);
    dppMock.dataContract.createFromBuffer.resolves(dataContract);

    const loggerMock = new LoggerMock(this.sinon);

    dataContractCacheMock = {
      set: this.sinon.stub(),
      get: this.sinon.stub(),
      has: this.sinon.stub(),
    };

    blockExecutionContextStackMock = {
      add: this.sinon.stub(),
    };

    blockExecutionContextStackRepositoryMock = {
      store: this.sinon.stub(),
    };

    groveDBStoreMock = {
      commitTransaction: this.sinon.stub(),
      getRootHash: this.sinon.stub().resolves(appHash),
    };

    commitHandler = commitHandlerFactory(
      creditsDistributionPoolMock,
      creditsDistributionPoolRepositoryMock,
      blockExecutionContextMock,
      blockExecutionContextStackMock,
      blockExecutionContextStackRepositoryMock,
      rotateSignedStoreMock,
      loggerMock,
      dataContractCacheMock,
      groveDBStoreMock,
    );
  });

  it('should commit db transactions, create document dbs and return ResponseCommit', async () => {
    const response = await commitHandler();

    expect(response).to.be.an.instanceOf(ResponseCommit);
    expect(response.data).to.deep.equal(appHash);

    expect(blockExecutionContextMock.getHeader).to.be.calledOnce();

    expect(creditsDistributionPoolMock.incrementAmount).to.be.calledOnceWith(
      accumulativeFees,
    );

    expect(creditsDistributionPoolRepositoryMock.store).to.be.calledOnceWith(
      creditsDistributionPoolMock,
      true,
    );

    expect(blockExecutionContextStackMock.add).to.be.calledOnceWith(
      blockExecutionContextMock,
    );

    expect(blockExecutionContextStackRepositoryMock.store).to.be.calledOnceWith(
      blockExecutionContextStackMock,
      true,
    );

    expect(groveDBStoreMock.commitTransaction).to.be.calledOnce();

    expect(blockExecutionContextMock.getDataContracts).to.be.calledOnce();

    expect(groveDBStoreMock.getRootHash).to.be.calledOnce();
  });
});
