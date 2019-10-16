const rewiremock = require('rewiremock/node');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const DataContractStateTransition = require('../../../lib/dataContract/stateTransition/DataContractStateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidStateTransitionError = require('../../../lib/stateTransition/errors/InvalidStateTransitionError');

const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('StateTransitionFactory', () => {
  let StateTransitionFactory;
  let decodeMock;
  let validateStateTransitionStructureMock;
  let createStateTransitionMock;
  let factory;
  let stateTransition;
  let rawStateTransition;

  beforeEach(function beforeEach() {
    const dataContract = getDataContractFixture();

    stateTransition = new DataContractStateTransition(dataContract);
    rawStateTransition = stateTransition.toJSON();

    decodeMock = this.sinonSandbox.stub();

    validateStateTransitionStructureMock = this.sinonSandbox.stub();
    createStateTransitionMock = this.sinonSandbox.stub().returns(stateTransition);

    // Require Factory module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/stateTransition/StateTransitionFactory');

    StateTransitionFactory = rewiremock.proxy('../../../lib/stateTransition/StateTransitionFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
    });

    factory = new StateTransitionFactory(
      validateStateTransitionStructureMock,
      createStateTransitionMock,
    );
  });

  describe('createFromObject', () => {
    it('should return new State Transition with data from passed object', () => {
      validateStateTransitionStructureMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawStateTransition);

      expect(result).to.equal(stateTransition);

      expect(validateStateTransitionStructureMock).to.have.been.calledOnceWith(rawStateTransition);

      expect(createStateTransitionMock).to.have.been.calledOnceWith(rawStateTransition);
    });

    it('should return new State Transition without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawStateTransition, { skipValidation: true });

      expect(result).to.equal(stateTransition);

      expect(validateStateTransitionStructureMock).to.have.not.been.called();

      expect(createStateTransitionMock).to.have.been.calledOnceWith(rawStateTransition);
    });

    it('should throw InvalidStateTransitionError if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateStateTransitionStructureMock.returns(new ValidationResult([validationError]));

      try {
        factory.createFromObject(rawStateTransition);

        expect.fail('InvalidStateTransitionError is not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidStateTransitionError);
        expect(e.getRawStateTransition()).to.equal(rawStateTransition);

        expect(e.getErrors()).to.have.length(1);

        const [consensusError] = e.getErrors();

        expect(consensusError).to.equal(validationError);

        expect(validateStateTransitionStructureMock).to.have.been.calledOnceWith(
          rawStateTransition,
        );

        expect(createStateTransitionMock).to.have.not.been.called();
      }
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new State Transition from serialized contract', () => {
      const serializedStateTransition = stateTransition.serialize();

      decodeMock.returns(rawStateTransition);

      factory.createFromObject.returns(stateTransition);

      const result = factory.createFromSerialized(serializedStateTransition);

      expect(result).to.equal(stateTransition);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawStateTransition);

      expect(decodeMock).to.have.been.calledOnceWith(serializedStateTransition);
    });
  });
});
