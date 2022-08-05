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
const GroveDBStoreMock = require('../../../../lib/test/mock/GroveDBStoreMock');
const BlockExecutionContextStackMock = require('../../../../lib/test/mock/BlockExecutionContextStackMock');
const BlockExecutionContextStackRepositoryMock = require('../../../../lib/test/mock/BlockExecutionContextStackRepositoryMock');

describe('commitHandlerFactory', () => {
  let commitHandler;
  let appHash;
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
  let executionTimerMock;

  beforeEach(function beforeEach() {
    appHash = Buffer.alloc(0);

    dataContract = getDataContractFixture();

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.getDataContracts.returns([dataContract]);
    blockExecutionContextMock.getCumulativeProcessingFee.returns(accumulativeFees);

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

    blockExecutionContextStackMock = new BlockExecutionContextStackMock(this.sinon);
    blockExecutionContextStackRepositoryMock = new BlockExecutionContextStackRepositoryMock(
      this.sinon,
    );

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    groveDBStoreMock.getRootHash.resolves(appHash);

    executionTimerMock = {
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    commitHandler = commitHandlerFactory(
      blockExecutionContextMock,
      blockExecutionContextStackMock,
      blockExecutionContextStackRepositoryMock,
      rotateSignedStoreMock,
      loggerMock,
      dataContractCacheMock,
      groveDBStoreMock,
      executionTimerMock,
    );
  });

  it('should commit db transactions, create document dbs and return ResponseCommit', async () => {
    const response = await commitHandler();

    expect(response).to.be.an.instanceOf(ResponseCommit);
    expect(response.data).to.deep.equal(appHash);

    expect(blockExecutionContextMock.getHeader).to.be.calledOnce();

    expect(blockExecutionContextStackMock.add).to.be.calledOnceWith(
      blockExecutionContextMock,
    );

    expect(blockExecutionContextStackRepositoryMock.store).to.be.calledOnceWith(
      blockExecutionContextStackMock,
      {
        useTransaction: true,
      },
    );

    expect(groveDBStoreMock.commitTransaction).to.be.calledOnce();

    expect(blockExecutionContextMock.getDataContracts).to.be.calledOnce();

    expect(groveDBStoreMock.getRootHash).to.be.calledOnce();
  });
});
