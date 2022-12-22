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

describe('deliverTxFactory', () => {
  let deliverTx;
  let documentTx;
  let dataContractTx;
  let identity;
  let dppMock;
  let stateRepositoryMock;
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
      total: 95,
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

    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    identity = getIdentityFixture();

    stateRepositoryMock.fetchIdentity.resolves(identity);

    dppMock.getStateRepository.returns(stateRepositoryMock);

    unserializeStateTransitionMock = this.sinon.stub();

    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    proposalBlockExecutionContextMock.getHeight.returns(42);

    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      getTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
      isStarted: this.sinon.stub(),
    };

    deliverTx = deliverTxFactory(
      unserializeStateTransitionMock,
      dppMock,
      proposalBlockExecutionContextMock,
      executionTimerMock,
    );
  });

  it('should apply a DocumentsBatchTransition and return ResponseDeliverTx', async () => {
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

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(
      documentsBatchTransitionFixture.toBuffer(),
    );
    expect(dppMock.stateTransition.validateState).to.be.calledOnceWith(
      documentsBatchTransitionFixture,
    );
    expect(dppMock.stateTransition.apply).to.be.calledOnceWith(
      documentsBatchTransitionFixture,
    );
    expect(proposalBlockExecutionContextMock.addDataContract).to.not.be.called();

    expect(stateRepositoryMock.fetchIdentity).to.be.calledOnceWith(
      documentsBatchTransitionFixture.getOwnerId(),
    );

    identity.reduceBalance(stateTransitionExecutionContextMock.getLastCalculatedFeeDetails().total);

    expect(stateRepositoryMock.updateIdentity).to.be.calledOnceWith(identity);
  });

  it('should apply a DataContractCreateTransition, add it to block execution state and return ResponseDeliverTx', async () => {
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

    expect(unserializeStateTransitionMock).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.toBuffer(),
    );
    expect(dppMock.stateTransition.validateState).to.be.calledOnceWith(
      dataContractCreateTransitionFixture,
    );
    expect(dppMock.stateTransition.apply).to.be.calledOnceWith(
      dataContractCreateTransitionFixture,
    );
    expect(proposalBlockExecutionContextMock.addDataContract).to.be.calledOnceWith(
      dataContractCreateTransitionFixture.getDataContract(),
    );

    expect(
      dataContractCreateTransitionFixture.getExecutionContext().dryOperations,
    ).to.have.length(0);
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
