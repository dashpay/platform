const Long = require('long');

const crypto = require('crypto');

const { FeeResult } = require('@dashevo/rs-drive');
const {
  DashPlatformProtocol,
  StateTransitionExecutionContext,
  ValidationResult,
  MissingStateTransitionTypeError,
} = require('@dashevo/wasm-dpp');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const getDataContractFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getDocumentsFixture');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const BlockExecutionContextMock = require('../../../../../lib/test/mock/BlockExecutionContextMock');

const deliverTxFactory = require('../../../../../lib/abci/handlers/proposal/deliverTxFactory');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const DPPValidationAbciError = require('../../../../../lib/abci/errors/DPPValidationAbciError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const StorageResult = require('../../../../../lib/storage/StorageResult');

describe('deliverTxFactory', () => {
  let deliverTx;
  let documentTx;
  let dataContractTx;
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
  let identityBalanceRepositoryMock;
  let processingFee;
  let storageFee;
  let refundsPerEpoch;
  let feeRefunds;
  let createContextLoggerMock;
  let calculateStateTransitionFeeMock;
  let calculateStateTransitionFeeFromOperationsMock;

  beforeEach(async function beforeEach() {
    round = 42;
    const dataContractFixture = await getDataContractFixture();
    const documentFixture = await getDocumentsFixture();

    const stateRepositoryMock = createStateRepositoryMock(this.sinon);

    dpp = new DashPlatformProtocol(this.blsAdapter, stateRepositoryMock, {
      generate: () => Buffer.alloc(32),
    });

    documentsBatchTransitionFixture = dpp.document.createStateTransition({
      create: documentFixture,
    });

    dataContractCreateTransitionFixture = dpp
      .dataContract.createDataContractCreateTransition(dataContractFixture);

    loggerMock = new LoggerMock(this.sinon);

    stateTransitionExecutionContextMock = new StateTransitionExecutionContext();

    processingFee = BigInt(10);
    storageFee = BigInt(100);
    const totalRefunds = BigInt(15);
    refundsPerEpoch = {
      1: totalRefunds,
    };
    const refundMap = new Map();
    refundMap.set('1', totalRefunds);
    feeRefunds = [
      {
        toObject: () => ({
          identifier: Buffer.alloc(32),
          creditsPerEpoch: refundMap,
        }),
      },
    ];

    const actualSTFees = FeeResult.create(storageFee, processingFee, feeRefunds);

    identityBalanceRepositoryMock = {
      applyFees: this.sinon.stub().resolves(
        new StorageResult(actualSTFees),
      ),
    };

    documentsBatchTransitionFixture.setExecutionContext(stateTransitionExecutionContextMock);
    dataContractCreateTransitionFixture.setExecutionContext(stateTransitionExecutionContextMock);

    documentTx = documentsBatchTransitionFixture.toBuffer();

    dataContractTx = dataContractCreateTransitionFixture.toBuffer();

    dppMock = createDPPMock(this.sinon);

    validationResult = new ValidationResult();

    dppMock
      .stateTransition
      .validateState
      .resolves(validationResult);

    unserializeStateTransitionMock = this.sinon.stub();

    proposalBlockExecutionContextMock = new BlockExecutionContextMock(this.sinon);
    proposalBlockExecutionContextMock.getHeight.returns(Long.fromNumber(42));
    proposalBlockExecutionContextMock.getEpochInfo.returns({
      currentEpochIndex: 0,
    });

    executionTimerMock = {
      clearTimer: this.sinon.stub(),
      getTimer: this.sinon.stub(),
      startTimer: this.sinon.stub(),
      stopTimer: this.sinon.stub(),
      isStarted: this.sinon.stub(),
    };

    createContextLoggerMock = this.sinon.stub().returns(loggerMock);

    calculateStateTransitionFeeMock = this.sinon.stub().returns({
      storageFee,
      processingFee,
      feeRefunds,
      totalRefunds,
      requiredAmount: processingFee - totalRefunds,
      desiredAmount: storageFee + processingFee - totalRefunds,
    });

    calculateStateTransitionFeeFromOperationsMock = this.sinon.stub().returns({
      storageFee,
      processingFee,
      feeRefunds,
      totalRefunds,
      requiredAmount: processingFee - totalRefunds,
      desiredAmount: storageFee + processingFee - totalRefunds,
    });

    deliverTx = deliverTxFactory(
      unserializeStateTransitionMock,
      dppMock,
      proposalBlockExecutionContextMock,
      executionTimerMock,
      identityBalanceRepositoryMock,
      calculateStateTransitionFeeMock,
      calculateStateTransitionFeeFromOperationsMock,
      createContextLoggerMock,
    );
  });

  it('should execute a state transition and return result', async () => {
    unserializeStateTransitionMock.resolves(documentsBatchTransitionFixture);

    const response = await deliverTx(documentTx, round, loggerMock);

    expect(response).to.deep.equal({
      code: 0,
      fees: {
        processingFee: Number(processingFee),
        storageFee: Number(storageFee),
        refundsPerEpoch: Object.entries(refundsPerEpoch).reduce((o, [key, value]) => ({
          [key]: Number(value),
          ...o,
        }), {}),
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

    expect(identityBalanceRepositoryMock.applyFees).to.be.calledOnce();

    const applyFeesToBalanceArgs = identityBalanceRepositoryMock.applyFees.getCall(0).args;

    expect(applyFeesToBalanceArgs).to.have.lengthOf(3);

    const identifier = applyFeesToBalanceArgs[0];

    expect(identifier).to.equals(documentsBatchTransitionFixture.getOwnerId());

    const feeResult = applyFeesToBalanceArgs[1];

    expect(feeResult).to.be.an.instanceOf(FeeResult);

    expect(feeResult.storageFee).to.equals(Number(storageFee));
    expect(feeResult.processingFee).to.equals(Number(processingFee));
    expect(feeResult.feeRefunds).to.deep.equals([
      {
        identifier: Buffer.alloc(32),
        creditsPerEpoch: {
          1: 15,
        },
      },
    ]);

    expect(applyFeesToBalanceArgs[2]).to.deep.equals({ useTransaction: true });

    expect(proposalBlockExecutionContextMock.addDataContract).to.not.be.called();

    expect(
      dataContractCreateTransitionFixture.getExecutionContext().getDryOperations(),
    ).to.have.length(0);

    const stHash = crypto
      .createHash('sha256')
      .update(documentTx)
      .digest()
      .toString('hex')
      .toUpperCase();
    expect(createContextLoggerMock).to.be.calledOnceWithExactly(loggerMock, {
      txId: stHash,
    });
  });

  it('should execute a DataContractCreateTransition', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const response = await deliverTx(dataContractTx, round, loggerMock);

    expect(response).to.deep.equal({
      code: 0,
      fees: {
        processingFee: Number(processingFee),
        storageFee: Number(storageFee),
        refundsPerEpoch: Object.entries(refundsPerEpoch).reduce((o, [key, value]) => ({
          [key]: Number(value),
          ...o,
        }), {}),
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

    expect(identityBalanceRepositoryMock.applyFees).to.be.calledOnce();

    const applyFeesToBalanceArgs = identityBalanceRepositoryMock.applyFees.getCall(0).args;

    expect(applyFeesToBalanceArgs).to.have.lengthOf(3);

    const identifier = applyFeesToBalanceArgs[0];

    expect(identifier).to.equals(dataContractCreateTransitionFixture.getOwnerId());

    const feeResult = applyFeesToBalanceArgs[1];

    expect(feeResult).to.be.an.instanceOf(FeeResult);

    expect(feeResult.storageFee).to.equals(Number(storageFee));
    expect(feeResult.processingFee).to.equals(Number(processingFee));
    expect(feeResult.feeRefunds).to.deep.equals([
      {
        identifier: Buffer.alloc(32),
        creditsPerEpoch: {
          1: 15,
        },
      },
    ]);

    expect(applyFeesToBalanceArgs[2]).to.deep.equals({ useTransaction: true });

    expect(
      dataContractCreateTransitionFixture.getExecutionContext().getDryOperations(),
    ).to.have.length(0);

    // expect(proposalBlockExecutionContextMock.addDataContract).to.be.calledOnceWithExactly(
    //   dataContractCreateTransitionFixture.getDataContract(),
    // );
  });

  it('should throw DPPValidationAbciError if a state transition is invalid against state', async () => {
    unserializeStateTransitionMock.resolves(dataContractCreateTransitionFixture);

    const error = new MissingStateTransitionTypeError();

    validationResult.addError(error.serialize());

    try {
      await deliverTx(documentTx, round, loggerMock);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        serializedError: error.serialize(),
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
});
