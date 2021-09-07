const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidStateTransitionError = require('../../../lib/stateTransition/errors/InvalidStateTransitionError');

const SerializedObjectParsingError = require('../../../lib/errors/consensus/basic/decode/SerializedObjectParsingError');

const createDPPMock = require('../../../lib/test/mocks/createDPPMock');
const StateTransitionFactory = require('../../../lib/stateTransition/StateTransitionFactory');
const SomeConsensusError = require('../../../lib/test/mocks/SomeConsensusError');

describe('StateTransitionFactory', () => {
  let validateStateTransitionBasicMock;
  let createStateTransitionMock;
  let factory;
  let stateTransition;
  let rawStateTransition;
  let decodeProtocolEntityMock;
  let dppMock;

  beforeEach(function beforeEach() {
    const dataContract = getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });
    rawStateTransition = stateTransition.toObject();

    decodeProtocolEntityMock = this.sinonSandbox.stub();

    validateStateTransitionBasicMock = this.sinonSandbox.stub();
    createStateTransitionMock = this.sinonSandbox.stub().returns(stateTransition);

    dppMock = createDPPMock();

    factory = new StateTransitionFactory(
      validateStateTransitionBasicMock,
      createStateTransitionMock,
      dppMock,
      decodeProtocolEntityMock,
    );
  });

  describe('createFromObject', () => {
    it('should return new State Transition with data from passed object', async () => {
      validateStateTransitionBasicMock.returns(new ValidationResult());

      const result = await factory.createFromObject(rawStateTransition);

      expect(result).to.equal(stateTransition);

      expect(validateStateTransitionBasicMock).to.have.been.calledOnceWith(rawStateTransition);

      expect(createStateTransitionMock).to.have.been.calledOnceWith(rawStateTransition);
    });

    it('should return new State Transition without validation if "skipValidation" option is passed', async () => {
      const result = await factory.createFromObject(rawStateTransition, { skipValidation: true });

      expect(result).to.equal(stateTransition);

      expect(validateStateTransitionBasicMock).to.have.not.been.called();

      expect(createStateTransitionMock).to.have.been.calledOnceWith(rawStateTransition);
    });

    it('should throw InvalidStateTransitionError if passed object is not valid', async () => {
      const validationError = new SomeConsensusError('test');

      validateStateTransitionBasicMock.returns(new ValidationResult([validationError]));

      try {
        await factory.createFromObject(rawStateTransition);

        expect.fail('InvalidStateTransitionError is not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidStateTransitionError);
        expect(e.getRawStateTransition()).to.equal(rawStateTransition);

        expect(e.getErrors()).to.have.length(1);

        const [consensusError] = e.getErrors();

        expect(consensusError).to.equal(validationError);

        expect(validateStateTransitionBasicMock).to.have.been.calledOnceWith(
          rawStateTransition,
        );
      }
    });
  });

  describe('createFromBuffer', () => {
    let serializedStateTransition;

    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');

      serializedStateTransition = stateTransition.toBuffer();
    });

    it('should return new State Transition from serialized contract', async () => {
      decodeProtocolEntityMock.returns([rawStateTransition.protocolVersion, rawStateTransition]);

      factory.createFromObject.resolves(stateTransition);

      const result = await factory.createFromBuffer(serializedStateTransition);

      expect(result).to.equal(stateTransition);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawStateTransition);

      expect(decodeProtocolEntityMock).to.have.been.calledOnceWith(
        serializedStateTransition,
        dppMock.getProtocolVersion(),
      );
    });

    it('should throw InvalidStateTransitionError if the decoding fails with consensus error', async () => {
      const parsingError = new SerializedObjectParsingError(
        serializedStateTransition,
        new Error(),
      );

      decodeProtocolEntityMock.throws(parsingError);

      try {
        await factory.createFromBuffer(serializedStateTransition);

        expect.fail('should throw InvalidStateTransitionError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidStateTransitionError);

        const [innerError] = e.getErrors();
        expect(innerError).to.be.equal(parsingError);
      }
    });

    it('should throw an error if decoding fails with any other error', async () => {
      const otherParsingError = new Error();

      decodeProtocolEntityMock.throws(otherParsingError);

      try {
        await factory.createFromBuffer(serializedStateTransition);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.equal(otherParsingError);
      }
    });
  });
});
