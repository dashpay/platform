const getIdentityCreateSTFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateSTFixture');

const ConsensusError = require('@dashevo/dpp/lib/errors/ConsensusError');
const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');

const unserializeStateTransitionFactory = require('../../../../../lib/abci/handlers/stateTransition/unserializeStateTransitionFactory');

const AbciError = require('../../../../../lib/abci/errors/AbciError');
const InvalidArgumentAbciError = require('../../../../../lib/abci/errors/InvalidArgumentAbciError');
const ExecutionTimedOutError = require('../../../../../lib/abci/errors/ExecutionTimedOutError');
const MemoryLimitExceededError = require('../../../../../lib/abci/errors/MemoryLimitExceededError');

describe('unserializeStateTransitionFactory', () => {
  let unserializeStateTransition;
  let stateTransitionFixture;
  let isolatedDppMock;
  let createIsolatedDppMock;

  beforeEach(function beforeEach() {
    stateTransitionFixture = getIdentityCreateSTFixture().serialize();

    isolatedDppMock = {
      dispose: this.sinon.stub(),
      stateTransition: {
        createFromSerialized: this.sinon.stub(),
      },
    };

    createIsolatedDppMock = this.sinon.stub().resolves(isolatedDppMock);

    unserializeStateTransition = unserializeStateTransitionFactory(createIsolatedDppMock);
  });

  it('should throw InvalidArgumentAbciError if State Transition is not specified', async () => {
    try {
      await unserializeStateTransition();

      expect.fail('should throw InvalidArgumentAbciError error');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidArgumentAbciError);
      expect(e.getMessage()).to.equal('State Transition is not specified');
      expect(e.getCode()).to.equal(AbciError.CODES.INVALID_ARGUMENT);

      expect(createIsolatedDppMock).to.not.be.called();
      expect(isolatedDppMock.dispose).to.not.be.called();
    }
  });

  it('should throw InvalidArgumentAbciError if State Transition is invalid', async () => {
    const consensusError = new ConsensusError('Invalid state transition');
    const error = new InvalidStateTransitionError(
      [consensusError],
      stateTransitionFixture.toJSON(),
    );

    isolatedDppMock.stateTransition.createFromSerialized.throws(error);

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

      expect(createIsolatedDppMock).to.be.calledOnce();
      expect(isolatedDppMock.dispose).to.be.calledOnce();
    }
  });

  it('should throw the error from createFromSerialized if throws not InvalidStateTransitionError', async () => {
    const error = new Error('Custom error');
    isolatedDppMock.stateTransition.createFromSerialized.throws(error);

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an error');
    } catch (e) {
      expect(e).to.be.equal(error);

      expect(createIsolatedDppMock).to.be.calledOnce();
      expect(isolatedDppMock.dispose).to.be.calledOnce();
    }
  });

  it('should throw a ExecutionTimedOutError if the VM Isolate execution timed out error thrown', async () => {
    const error = new Error('Script execution timed out.');
    isolatedDppMock.stateTransition.createFromSerialized.throws(error);

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an ExecutionTimedOutError');
    } catch (e) {
      expect(e).to.be.instanceOf(ExecutionTimedOutError);

      expect(createIsolatedDppMock).to.be.calledOnce();
      expect(isolatedDppMock.dispose).to.be.calledOnce();
    }
  });

  it('should throw a MemoryLimitExceededError if the VM Isolate memory limit exceeded error thrown', async () => {
    const error = new Error('Isolate was disposed during execution due to memory limit');
    isolatedDppMock.stateTransition.createFromSerialized.throws(error);

    try {
      await unserializeStateTransition(stateTransitionFixture);

      expect.fail('should throw an ExecutionTimedOutError');
    } catch (e) {
      expect(e).to.be.instanceOf(MemoryLimitExceededError);

      expect(createIsolatedDppMock).to.be.calledOnce();
      expect(isolatedDppMock.dispose).to.be.calledOnce();
    }
  });

  it('should return stateTransition', async () => {
    const stateTransition = getIdentityCreateSTFixture();

    isolatedDppMock.stateTransition.createFromSerialized.resolves(stateTransition);

    const result = await unserializeStateTransition(stateTransitionFixture);

    expect(result).to.deep.equal(stateTransition);
  });
});
