const Long = require('long');

const endBlockFactory = require('../../../../../lib/abci/handlers/proposal/endBlockFactory');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const GroveDBStoreMock = require('../../../../../lib/test/mock/GroveDBStoreMock');

describe('endBlockFactory', () => {
  let endBlock;
  let height;
  let blockExecutionContextMock;
  let dpnsContractBlockHeight;
  let loggerMock;
  let createValidatorSetUpdateMock;
  let validatorSetMock;
  let getFeatureFlagForHeightMock;
  let rsAbciMock;
  let blockEndMock;
  let time;
  let updateConsensusParamsMock;
  let rotateValidatorsMock;
  let groveDBStoreMock;
  let appHashFixture;
  let validatorSetUpdateFixture;
  let consensusParamUpdatesFixture;
  let processingFees;
  let storageFees;

  beforeEach(function beforeEach() {
    time = {
      seconds: Math.ceil(new Date().getTime() / 1000),
      nanos: 0,
    };

    blockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    blockExecutionContextMock.hasDataContract.returns(true);
    blockExecutionContextMock.getTime.returns(time);

    loggerMock = new LoggerMock(this.sinon);

    dpnsContractBlockHeight = 2;

    validatorSetMock = {
      rotate: this.sinon.stub(),
      getQuorum: this.sinon.stub(),
    };

    createValidatorSetUpdateMock = this.sinon.stub();

    getFeatureFlagForHeightMock = this.sinon.stub().resolves(null);

    blockEndMock = this.sinon.stub();

    rsAbciMock = {
      blockEnd: blockEndMock,
    };

    blockEndMock.resolves({
      currentEpochIndex: 42,
      isEpochChange: true,
    });

    processingFees = 43;
    storageFees = 44;

    consensusParamUpdatesFixture = Buffer.alloc(1);
    validatorSetUpdateFixture = Buffer.alloc(2);
    appHashFixture = Buffer.alloc(0);

    updateConsensusParamsMock = this.sinon.stub().resolves(consensusParamUpdatesFixture);
    rotateValidatorsMock = this.sinon.stub().resolves(validatorSetUpdateFixture);

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    groveDBStoreMock.getRootHash.resolves(appHashFixture);

    endBlock = endBlockFactory(
      blockExecutionContextMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      getFeatureFlagForHeightMock,
      updateConsensusParamsMock,
      rotateValidatorsMock,
      rsAbciMock,
      groveDBStoreMock,
    );

    height = Long.fromInt(dpnsContractBlockHeight);
  });

  it('should finalize a block', async () => {
    const response = await endBlock(height, processingFees, storageFees, loggerMock);

    expect(response).to.deep.equal({
      consensusParamUpdates: consensusParamUpdatesFixture,
      validatorSetUpdate: validatorSetUpdateFixture,
      appHash: appHashFixture,
    });

    expect(blockExecutionContextMock.hasDataContract).to.not.have.been.called();
    expect(updateConsensusParamsMock).to.be.calledOnceWithExactly(height, loggerMock);
    expect(rotateValidatorsMock).to.be.calledOnceWithExactly(height, loggerMock);
    expect(groveDBStoreMock.getRootHash).to.be.calledOnceWithExactly({ useTransaction: true });
    expect(rsAbciMock.blockEnd).to.be.calledOnceWithExactly({
      fees: {
        processingFees,
        storageFees,
      },
    }, true);
  });
});
