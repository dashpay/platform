const rewiremock = require('rewiremock/node');

const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDPContractError = require('../../../lib/contract/errors/InvalidDPContractError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('DPContractFactory', () => {
  let DPContractFactory;
  let decodeMock;
  let validateDPContractMock;
  let createDPContractMock;
  let factory;
  let dpContract;
  let rawDPContract;

  beforeEach(function beforeEach() {
    dpContract = getDPContractFixture();
    rawDPContract = dpContract.toJSON();

    decodeMock = this.sinonSandbox.stub();
    validateDPContractMock = this.sinonSandbox.stub();
    createDPContractMock = this.sinonSandbox.stub().returns(dpContract);

    // Require Factory module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/contract/DPContractFactory');

    DPContractFactory = rewiremock.proxy('../../../lib/contract/DPContractFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
    });

    factory = new DPContractFactory(
      createDPContractMock,
      validateDPContractMock,
    );
  });

  describe('create', () => {
    it('should return new DPContract with specified name and objects definition', () => {
      const result = factory.create(
        rawDPContract.name,
        rawDPContract.dpObjectsDefinition,
      );

      expect(result).to.be.equal(dpContract);

      expect(createDPContractMock).to.be.calledOnceWith({
        name: rawDPContract.name,
        dpObjectsDefinition: rawDPContract.dpObjectsDefinition,
      });
    });
  });

  describe('createFromObject', () => {
    it('should return new DPContract with data from passed object', () => {
      validateDPContractMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDPContract);

      expect(result).to.be.equal(dpContract);

      expect(validateDPContractMock).to.be.calledOnceWith(rawDPContract);

      expect(createDPContractMock).to.be.calledOnceWith(rawDPContract);
    });

    it('should return new DPContract without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawDPContract, { skipValidation: true });

      expect(result).to.be.equal(dpContract);

      expect(validateDPContractMock).not.to.be.called();

      expect(createDPContractMock).to.be.calledOnceWith(rawDPContract);
    });

    it('should throw error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDPContractMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDPContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDPContractError);
      expect(error.getRawDPContract()).to.be.equal(rawDPContract);

      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.be.equal(validationError);

      expect(validateDPContractMock).to.be.calledOnceWith(rawDPContract);

      expect(createDPContractMock).not.to.be.called();
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new DPContract from serialized DPContract', () => {
      const serializedDPContract = dpContract.serialize();

      decodeMock.returns(rawDPContract);

      factory.createFromObject.returns(dpContract);

      const result = factory.createFromSerialized(serializedDPContract);

      expect(result).to.be.equal(dpContract);

      expect(factory.createFromObject).to.be.calledOnceWith(rawDPContract);

      expect(decodeMock).to.be.calledOnceWith(serializedDPContract);
    });
  });
});
