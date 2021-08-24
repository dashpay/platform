const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const ConsensusError = require('@dashevo/dpp/lib/errors/ConsensusError');
const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');
const BalanceNotEnoughError = require('@dashevo/dpp/lib/errors/BalanceIsNotEnoughError');
const ValidatorResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const unserializeStateTransitionFactory = require('../../../../../lib/abci/handlers/stateTransition/unserializeStateTransitionFactory');

const AbciError = require('../../../../../lib/abci/errors/AbciError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const InsufficientFundsError = require('../../../../../lib/abci/errors/InsufficientFundsError');
const LoggerMock = require('../../../../../lib/test/mock/LoggerMock');

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
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);

      expect(dppMock.stateTransition.validateFee).to.not.be.called();
    }
  });

  it('should throw InvalidArgumentAbciError if State Transition is invalid', async () => {
    const consensusError = new ConsensusError('Invalid state transition');
    const error = new InvalidStateTransitionError(
      [consensusError],
      stateTransitionFixture,
    );

    dppMock.stateTransition.createFromBuffer.throws(error);

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getMessage()).to.equal('State Transition is invalid');
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);
      expect(e.getData()).to.deep.equal({
        errors: [consensusError],
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
    const error = new BalanceNotEnoughError(balance);

    dppMock.stateTransition.validateFee.resolves(
      new ValidatorResult([error]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(InsufficientFundsError);
      expect(e.getData().balance).to.equal(balance);

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.be.calledOnce();
    }
  });

  it('should return invalid result if validateSignature failed', async () => {
    const error = new Error('identity was not found');

    dppMock.stateTransition.validateSignature.resolves(
      new ValidatorResult([error]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getData().errors[0]).to.equal(error);

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
    const error = new BalanceNotEnoughError(balance);

    dppMock.stateTransition.validateFee.resolves(
      new ValidatorResult([error]),
    );

    try {
      await unserializeStateTransition(stateTransitionFixture, { logger: loggerMock });

      expect.fail('should throw an InsufficientFundsError');
    } catch (e) {
      expect(e).to.be.instanceOf(InsufficientFundsError);
      expect(e.getData().balance).to.equal(balance);

      expect(dppMock.stateTransition.createFromBuffer).to.be.calledOnce();
      expect(dppMock.stateTransition.validateFee).to.be.calledOnce();

      expect(noopLoggerMock.info).to.not.have.been.called();
      expect(noopLoggerMock.debug).to.not.have.been.called();

      expect(loggerMock.info).to.have.been.calledOnceWithExactly(
        'Insufficient funds to process state transition',
      );
      expect(loggerMock.debug).to.have.been.calledOnceWithExactly({
        consensusErrors: [error],
      });
    }
  });
});
