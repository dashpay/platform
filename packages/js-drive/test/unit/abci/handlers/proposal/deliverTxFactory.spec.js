const Long = require('long');

const DashPlatformProtocol = require('@dashevo/dpp');

const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');
const deliverTxFactory = require('../../../../../lib/abci/handlers/proposal/deliverTxFactory');

const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const DPPValidationAbciError = require('../../../../../lib/abci/errors/DPPValidationAbciError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const PredictedFeeLowerThanActualError = require('../../../../../lib/abci/handlers/errors/PredictedFeeLowerThanActualError');
const NegativeBalanceError = require('../../../../../lib/abci/handlers/errors/NegativeBalanceError');
const BlockInfo = require('../../../../../lib/blockExecution/BlockInfo');

describe('deliverTxFactory', () => {
  let deliverTx;
  let documentTx;
  let dataContractTx;
  let identity;
  let dppMock;
  let documentsBatchTransitionFixture;
  let dataContractCreateTransitionFixture;
  let dpp;
  let unserializeStateTransitionMock;
  let validationResult;
  let executionTimerMock;
  let loggerMock;
  let round;
  let proposalBlockExecutionContextMock;
  let stateTransitionExecutionContextMock;
  let identityRepositoryMock;
  let blockInfo;

  beforeEach(async function beforeEach() {
    round = 42;
    const dataContractFixture = getDataContractFixture();
    const documentFixture = getDocumentFixture();

    loggerMock = new LoggerMock(this.sinon);

    dpp = new DashPlatformProtocol();
    await dpp.initialize();

    stateTransitionExecutionContextMock = new StateTransitionExecutionContext();

    stateTransitionExecutionContextMock.setLastCalculatedFeeDetails({
      storageFee: 100,
      processingFee: 10,
      feeRefunds: [{ identifier: Buffer.alloc(32), creditsPerEpoch: { 1: 15 } }],
      feeRefundsSum: 15,
      requiredAmount: 85,
      desiredAmount: 95,
    });

    documentsBatchTransitionFixture = dpp.document.createStateTransition({
      create: documentFixture,
    });

    documentsBatchTransitionFixture.setExecutionContext(stateTransitionExecutionContextMock);

    dataContractCreateTransitionFixture = dpp
      .dataContract.createDataContractCreateTransition(dataContractFixture);

    dataContractCreateTransitionFixture.setExecutionContext(stateTransitionExecutionContextMock);

    documentTx = documentsBatchTransitionFixture.toBuffer();

    dataContractTx = dataContractCreateTransitionFixture.toBuffer();

    dppMock = createDPPMock(this.sinon);

    validationResult = new ValidationResult();

    dppMock
      .stateTransition
      .validateState
      .resolves(validationResult);

    identity = getIdentityFixture();

    unserializeStateTransitionMock = this.sinon.stub();

    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    proposalBlockExecutionContextMock.getHeight.returns(Long.fromNumber(42));
    proposalBlockExecutionContextMock.getEpochInfo.returns({
      currentEpochIndex: 0,
    });

    blockInfo = BlockInfo.createFromBlockExecutionContext(proposalBlockExecutionContextMock);

    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      getTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
      isStarted: this.sinon.stub(),
    };

    identityRepositoryMock = {
      addToBalance: this.sinon.stub(),
      removeFromBalance: this.sinon.stub(),
    };

    deliverTx = deliverTxFactory(
      unserializeStateTransitionMock,
      dppMock,
      proposalBlockExecutionContextMock,
      executionTimerMock,
      identityRepositoryMock,
    );
  });

  it('should execute a DocumentsBatchTransition', async () => {
    unserializeStateTransitionMock.resolves(documentsBatchTransitionFixture);

    const response = await deliverTx(documentTx, round, loggerMock);

    expect(response).to.deep.equal({
      code: 0,
      fees: {
        processingFee: 10,
        storageFee: 100,
        feeRefunds: {
          1: 15,
        },
        feeRefundsSum: 15,
      },
    });

    expect(unserializeStateTransitionMock).to.be.calledOnceWithExactly(
      documentsBatchTransitionFixture.toBuffer(),
      {
        logger: loggerMock,
        executionTimer: executionTimerMock,
      },
    );

    expect(dppMock.stateTransition.validateState).to.be.calledOnceWithExactly(
      documentsBatchTransitionFixture,
    );

    expect(dppMock.stateTransition.apply).to.be.calledOnceWithExactly(
      documentsBatchTransitionFixture,
    );

    expect(identityRepositoryMock.removeFromBalance).to.be.calledOnceWithExactly(
      documentsBatchTransitionFixture.getOwnerId(),
      85,
      95,
      blockInfo,
      { useTransaction: true },
    );

    expect(identityRepositoryMock.addToBalance).to.not.be.called();

    expect(proposalBlockExecutionContextMock.addDataContract).to.not.be.called();

    expect(
      dataContractCreateTransitionFixture.getExecutionContext().dryOperations,
    ).to.have.length(0);
  });

  it('should execute a DataContractCreateTransition', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const response = await deliverTx(dataContractTx, round, loggerMock);

    expect(response).to.deep.equal({
      code: 0,
      fees: {
        processingFee: 10,
        storageFee: 100,
        feeRefunds: {
          1: 15,
        },
        feeRefundsSum: 15,
      },
    });

    expect(unserializeStateTransitionMock).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture.toBuffer(),
      {
        logger: loggerMock,
        executionTimer: executionTimerMock,
      },
    );

    expect(dppMock.stateTransition.validateState).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture,
    );

    expect(dppMock.stateTransition.apply).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture,
    );

    expect(identityRepositoryMock.removeFromBalance).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture.getOwnerId(),
      85,
      95,
      blockInfo,
      { useTransaction: true },
    );

    expect(identityRepositoryMock.addToBalance).to.not.be.called();

    expect(
      dataContractCreateTransitionFixture.getExecutionContext().dryOperations,
    ).to.have.length(0);

    expect(proposalBlockExecutionContextMock.addDataContract).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.getDataContract(),
    );
  });

  it('should add to balance if refunds are higher than storage and processing fees', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    stateTransitionExecutionContextMock.setLastCalculatedFeeDetails({
      storageFee: 10,
      processingFee: 10,
      feeRefunds: [{ identifier: Buffer.alloc(32), creditsPerEpoch: { 1: 15, 2: 10 } }],
      feeRefundsSum: 25,
      requiredAmount: -15,
      desiredAmount: -5,
    });

    const response = await deliverTx(dataContractTx, round, loggerMock);

    expect(response).to.deep.equal({
      code: 0,
      fees: {
        processingFee: 10,
        storageFee: 10,
        feeRefunds: {
          1: 15,
          2: 10,
        },
        feeRefundsSum: 25,
      },
    });

    expect(unserializeStateTransitionMock).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture.toBuffer(),
      {
        logger: loggerMock,
        executionTimer: executionTimerMock,
      },
    );

    expect(dppMock.stateTransition.validateState).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture,
    );

    expect(dppMock.stateTransition.apply).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture,
    );

    expect(identityRepositoryMock.addToBalance).to.be.calledOnceWithExactly(
      dataContractCreateTransitionFixture.getOwnerId(),
      5,
      blockInfo,
      { useTransaction: true },
    );

    expect(identityRepositoryMock.removeFromBalance).to.not.be.called();

    expect(
      dataContractCreateTransitionFixture.getExecutionContext().dryOperations,
    ).to.have.length(0);

    expect(proposalBlockExecutionContextMock.addDataContract).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.getDataContract(),
    );
  });

  it('should throw DPPValidationAbciError if a state transition is invalid against state', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const error = new SomeConsensusError('Consensus error');

    validationResult.addError(error);

    try {
      await deliverTx(documentTx, round, loggerMock);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        arguments: ['Consensus error'],
      });
    }
  });

  it('should throw DPPValidationAbciError if a state transition is not valid', async () => {
    const errorMessage = 'Invalid structure';
    const error = new InvalidArgumentAbciError(errorMessage);

    unserializeStateTransitionMock.throws(error);

    try {
      await deliverTx(documentTx, round, loggerMock);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getMessage()).to.equal(errorMessage);
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
      expect(dppMock.stateTransition.validate).to.not.be.called();
    }
  });

  // TODO: enable once fee calculation is done
  it.skip('should throw PredictedFeeLowerThanActualError if actual fee > predicted fee', async function it() {
    dataContractCreateTransitionFixture.calculateFee = this.sinon.stub().returns(0);

    dataContractCreateTransitionFixture.calculateFee.onCall(1).returns(10);

    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    try {
      await deliverTx(documentTx, round, loggerMock);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(PredictedFeeLowerThanActualError);
      expect(e.getStateTransition().toBuffer())
        .to.deep.equal(dataContractCreateTransitionFixture.toBuffer());
    }
  });

  // TODO: enable once fee calculation is done
  it.skip('should throw NegativeBalanceError if balance < fee', async function it() {
    dataContractCreateTransitionFixture.calculateFee = this.sinon.stub().returns(0);

    dataContractCreateTransitionFixture.calculateFee.returns(100);

    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    try {
      await deliverTx(documentTx, round, loggerMock);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(NegativeBalanceError);
    }
  });
});
