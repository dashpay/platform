const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const InvalidStateTransitionTypeError = require('@dashevo/dpp/lib/errors/consensus/basic/stateTransition/InvalidStateTransitionTypeError');
const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');
const BalanceNotEnoughError = require('@dashevo/dpp/lib/errors/consensus/fee/BalanceIsNotEnoughError');
const ValidatorResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const IdentityNotFoundError = require('@dashevo/dpp/lib/errors/consensus/signature/IdentityNotFoundError');
const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const unserializeStateTransitionFactory = require('../../../../../lib/abci/handlers/stateTransition/unserializeStateTransitionFactory');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');
const DPPValidationAbciError = require('../../../../../lib/abci/errors/DPPValidationAbciError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');

describe('unserializeStateTransitionFactory', () => {
  let unserializeStateTransition;
  let stateTransitionFixture;
  let dppMock;
  let noopLoggerMock;

  beforeEach(function beforeEach() {
    stateTransitionFixture = getIdentityCreateTransitionFixture().toBuffer();

    dppMock = {
      dispose: this.sinon.stub(),
      stateTransition: {
        createFromBuffer: this.sinon.stub(),
        validateFee: this.sinon.stub(),
        validateSignature: this.sinon.stub(),
      },
    };

    dppMock.stateTransition.validateSignature.resolves(new ValidatorResult());

    noopLoggerMock = new LoggerMock(this.sinon);

    unserializeStateTransition = unserializeStateTransitionFactory(dppMock, noopLoggerMock);
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
      [dppError],
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
        arguments: [-1],
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
    const balance = 1000;
    const fee = 1;
    const error = new BalanceNotEnoughError(balance, fee);

    dppMock.stateTransition.validateFee.resolves(
      new ValidatorResult([error]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        arguments: [balance, fee],
      });

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.be.calledOnce();
    }
  });

  it('should return invalid result if validateSignature failed', async () => {
    const identity = getIdentityFixture();
    const error = new IdentityNotFoundError(identity.getId());

    dppMock.stateTransition.validateSignature.resolves(
      new ValidatorResult([error]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        arguments: [identity.getId()],
      });

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.have.not.been.called();
    }
  });

  it('should return stateTransition', async () => {
    const stateTransition = getIdentityCreateTransitionFixture();

    dppMock.stateTransition.createFromBuffer.resolves(stateTransition);

    dppMock.stateTransition.validateFee.resolves(new ValidatorResult());

    const result = await unserializeStateTransition(stateTransitionFixture);

    expect(result).to.deep.equal(stateTransition);

    expect(dppMock.stateTransition.validateFee).to.be.calledOnceWith(stateTransition);
  });

  it('should use provided logger', async function it() {
    const loggerMock = new LoggerMock(this.sinon);

    const balance = 1000;
    const fee = 1000;
    const error = new BalanceNotEnoughError(balance, fee);

    dppMock.stateTransition.validateFee.resolves(
      new ValidatorResult([error]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture, { logger: loggerMock });

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(DPPValidationAbciError);
      expect(e.getCode()).to.equal(error.getCode());
      expect(e.getData()).to.deep.equal({
        arguments: [balance, fee],
      });

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.be.calledOnce();

      expect(noopLoggerMock.info).to.not.have.been.called();
      expect(noopLoggerMock.debug).to.not.have.been.called();

      expect(loggerMock.info).to.have.been.calledOnceWithExactly(
        'Insufficient funds to process state transition',
      );
      expect(loggerMock.debug).to.have.been.calledOnceWithExactly({
        consensusError: error,
      });
    }
  });
});
