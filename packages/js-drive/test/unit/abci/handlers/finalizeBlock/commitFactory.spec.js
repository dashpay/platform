const Long = require('long');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');

const commitFactory = require('../../../../../lib/abci/handlers/finalizeBlock/commitFactory');

const RootTreeMock = require('../../../../../lib/test/mock/RootTreeMock');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const GroveDBStoreMock = require('../../../../../lib/test/mock/GroveDBStoreMock');
const BlockExecutionContextStackMock = require('../../../../../lib/test/mock/BlockExecutionContextStackMock');
const BlockExecutionContextStackRepositoryMock = require('../../../../../lib/test/mock/BlockExecutionContextStackRepositoryMock');

describe('commitFactory', () => {
  let commit;
  let appHash;
  let creditsDistributionPoolMock;
  let creditsDistributionPoolRepositoryMock;
  let blockExecutionContextMock;
  let dataContract;
  let accumulativeFees;
  let rootTreeMock;
  let dppMock;
  let height;
  let dataContractCacheMock;
  let blockExecutionContextStackMock;
  let blockExecutionContextStackRepositoryMock;
  let groveDBStoreMock;
  let rotateSignedStoreMock;
  let loggerMock;

  beforeEach(function beforeEach() {
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

    height = Long.fromInt(1);

    blockExecutionContextMock.getHeight.returns(height);

    rootTreeMock = new RootTreeMock(this.sinon);
    rootTreeMock.getRootHash.returns(appHash);

    dppMock = createDPPMock(this.sinon);
    dppMock.dataContract.createFromBuffer.resolves(dataContract);

    loggerMock = new LoggerMock(this.sinon);

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

    commit = commitFactory(
      creditsDistributionPoolMock,
      creditsDistributionPoolRepositoryMock,
      blockExecutionContextMock,
      blockExecutionContextStackMock,
      blockExecutionContextStackRepositoryMock,
      rotateSignedStoreMock,
      dataContractCacheMock,
      groveDBStoreMock,
    );
  });

  it('should commit db transactions, create document dbs and return ResponseCommit', async () => {
    const response = await commit(loggerMock);

    expect(response).to.deep.equal({ appHash });

    expect(blockExecutionContextMock.getHeight).to.be.calledOnce();

    expect(creditsDistributionPoolMock.incrementAmount).to.be.calledOnceWith(
      accumulativeFees,
    );

    expect(creditsDistributionPoolRepositoryMock.store).to.be.calledOnceWith(
      creditsDistributionPoolMock,
      {
        useTransaction: true,
      },
    );

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
