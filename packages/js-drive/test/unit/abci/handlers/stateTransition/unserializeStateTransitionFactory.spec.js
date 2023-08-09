const {
  InvalidStateTransitionTypeError,
  InvalidStateTransitionError,
  BalanceIsNotEnoughError,
  ValidationResult,
  IdentityNotFoundError,
} = require('@dashevo/wasm-dpp');
const getIdentityCreateTransitionFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const getIdentityFixture = require('@dashevo/wasm-dpp/lib/test/fixtures/getIdentityFixture');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const unserializeStateTransitionFactory = require('../../../../../lib/abci/handlers/stateTransition/unserializeStateTransitionFactory');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const DPPValidationAbciError = require('../../../../../lib/abci/errors/DPPValidationAbciError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');

describe('unserializeStateTransitionFactory', () => {
  let unserializeStateTransition;
  let stateTransitionFixture;
  let dppMock;
  let noopLoggerMock;
  let stateTransition;

  beforeEach(async function beforeEach() {
    stateTransition = await getIdentityCreateTransitionFixture();
    stateTransitionFixture = stateTransition.toBuffer();

    dppMock = {
      dispose: this.sinon.stub(),
      stateTransition: {
        createFromBuffer: this.sinon.stub(),
        validateFee: this.sinon.stub(),
        validateSignature: this.sinon.stub(),
        validateState: this.sinon.stub(),
        apply: this.sinon.stub(),
      },
    };

    dppMock.stateTransition.validateSignature.resolves(new ValidationResult());

    noopLoggerMock = new LoggerMock(this.sinon);

    unserializeStateTransition = unserializeStateTransitionFactory(
      dppMock, noopLoggerMock,
    );
  });

  it('should throw InvalidArgumentAbciError if State Transition is not specified', async () => {
    try {
      await unserializeStateTransition();

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getMessage()).to.equal('State Transition is not specified');
      expect(e.getCode()).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);

      expect(dppMock.stateTransition.validateFee).to.not.be.called();
    }
  });

  it('should throw InvalidArgumentAbciError if State Transition is invalid', async () => {
    const dppError = new InvalidStateTransitionTypeError(-1);
    const error = new InvalidStateTransitionError(
      [dppError.serialize()],
      stateTransitionFixture,
    );

    dppMock.stateTransition.createFromBuffer.throws(error);

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(dppError.getCode());
      expect(e.getData()).to.deep.equal({
        serializedError: dppError.serialize(),
      });

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.not.be.called();
    }
  });

  it('should throw the error from createFromBuffer if throws not InvalidStateTransitionError', async () => {
    const error = new Error('Custom error');
    dppMock.stateTransition.createFromBuffer.throws(error);

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.equal(error);

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.not.be.called();
    }
  });

  it('should throw InsufficientFundsError in case if identity has not enough credits', async () => {
    const balance = BigInt(1000);
    const fee = BigInt(1);
    const error = new BalanceIsNotEnoughError(balance, fee);

    dppMock.stateTransition.validateFee.resolves(
      new ValidationResult([error.serialize()]),
    );

    dppMock.stateTransition.createFromBuffer.resolves(stateTransition);

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        serializedError: error.serialize(),
      });

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.be.calledOnce();
    }
  });

  it('should return invalid result if validateSignature failed', async () => {
    const identity = await getIdentityFixture();
    const error = new IdentityNotFoundError(identity.getId());

    dppMock.stateTransition.validateSignature.resolves(
      new ValidationResult([error.serialize()]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        serializedError: error.serialize(),
      });

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.have.not.been.called();
    }
  });

  it('should return stateTransition', async () => {
    dppMock.stateTransition.createFromBuffer.resolves(stateTransition);

    dppMock.stateTransition.validateFee.resolves(new ValidationResult());

    const result = await unserializeStateTransition(stateTransitionFixture);

    expect(result).to.deep.equal(stateTransition);

    // TODO: Enable fee validation when RS Drive is ready
    // expect(dppMock.stateTransition.validateFee).to.be.calledOnceWith(stateTransition);
    // expect(dppMock.stateTransition.validateState).to.be.calledOnceWithExactly(stateTransition);
    // expect(dppMock.stateTransition.apply).to.be.calledOnceWithExactly(stateTransition);
  });

  it('should use provided logger', async function it() {
    const loggerMock = new LoggerMock(this.sinon);

    const balance = BigInt(1000);
    const fee = BigInt(1000);
    const error = new BalanceIsNotEnoughError(balance, fee);

    dppMock.stateTransition.createFromBuffer.resolves(stateTransition);

    dppMock.stateTransition.validateFee.resolves(
      new ValidationResult([error.serialize()]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture, { logger: loggerMock });

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        serializedError: error.serialize(),
      });

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.be.calledOnce();

      expect(noopLoggerMock.info).to.not.have.been.called();
      expect(noopLoggerMock.debug).to.not.have.been.called();

      expect(loggerMock.info).to.have.been.calledOnceWithExactly(
        'Insufficient funds to process state transition',
      );
      // expect(loggerMock.debug).to.have.been.calledOnceWithExactly({
      //   consensusError: error,
      // });
    }
  });
});
