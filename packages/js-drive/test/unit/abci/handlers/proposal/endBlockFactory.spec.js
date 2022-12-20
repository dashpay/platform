const Long = require('long');

const FeeResult = require('@dashevo/rs-drive/FeeResult');

const endBlockFactory = require('../../../../../lib/abci/handlers/proposal/endBlockFactory');
const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const GroveDBStoreMock = require('../../../../../lib/test/mock/GroveDBStoreMock');

describe('endBlockFactory', () => {
  let endBlock;
  let height;
  let dpnsContractBlockHeight;
  let loggerMock;
  let createValidatorSetUpdateMock;
  let validatorSetMock;
  let getFeatureFlagForHeightMock;
  let rsAbciMock;
  let blockEndMock;
  let time;
  let createConsensusParamUpdateMock;
  let rotateAndCreateValidatorSetUpdateMock;
  let groveDBStoreMock;
  let appHashFixture;
  let validatorSetUpdateFixture;
  let consensusParamUpdatesFixture;
  let executionTimerMock;
  let proposalBlockExecutionContextMock;
  let round;
  let coreChainLockedHeight;
  let fees;

  beforeEach(function beforeEach() {
    round = 42;
    coreChainLockedHeight = 41;
    time = Date.now();
    fees = FeeResult.create(1, 2);

    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
    };

    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);

    proposalBlockExecutionContextMock.hasDataContract.returns(true);
    proposalBlockExecutionContextMock.getTimeMs.returns(time);

    proposalBlockExecutionContextMock.getEpochInfo.returns({
      currentEpochIndex: 42,
      isEpochChange: true,
    });

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

    consensusParamUpdatesFixture = Buffer.alloc(1);
    validatorSetUpdateFixture = Buffer.alloc(2);
    appHashFixture = Buffer.alloc(0);

    createConsensusParamUpdateMock = this.sinon.stub().resolves(consensusParamUpdatesFixture);
    rotateAndCreateValidatorSetUpdateMock = this.sinon.stub().resolves(validatorSetUpdateFixture);

    groveDBStoreMock = new GroveDBStoreMock(this.sinon);
    groveDBStoreMock.getRootHash.resolves(appHashFixture);

    endBlock = endBlockFactory(
      proposalBlockExecutionContextMock,
      validatorSetMock,
      createValidatorSetUpdateMock,
      getFeatureFlagForHeightMock,
      createConsensusParamUpdateMock,
      rotateAndCreateValidatorSetUpdateMock,
      rsAbciMock,
      groveDBStoreMock,
      executionTimerMock,
    );

    height = Long.fromInt(dpnsContractBlockHeight);
  });

  it('should end block', async () => {
    const response = await endBlock({
      height, round, fees, coreChainLockedHeight,
    }, loggerMock);

    expect(response).to.deep.equal({
      consensusParamUpdates: consensusParamUpdatesFixture,
      validatorSetUpdate: validatorSetUpdateFixture,
      appHash: appHashFixture,
    });

    expect(proposalBlockExecutionContextMock.hasDataContract).to.not.have.been.called();
    expect(createConsensusParamUpdateMock).to.be.calledOnceWithExactly(height, round, loggerMock);
    expect(rotateAndCreateValidatorSetUpdateMock).to.be.calledOnceWithExactly(
      height,
      coreChainLockedHeight,
      round,
      loggerMock,
    );
    expect(groveDBStoreMock.getRootHash).to.be.calledOnceWithExactly({ useTransaction: true });

    expect(rsAbciMock.blockEnd).to.be.calledOnceWithExactly({ fees }, true);

    const { fees: actualFees } = rsAbciMock.blockEnd.getCall(0).args[0];

    expect(actualFees.storageFee).to.equal(1);
    expect(actualFees.processingFee).to.equal(2);

    expect(executionTimerMock.stopTimer).to.be.calledOnceWithExactly('roundExecution', true);
  });
});
