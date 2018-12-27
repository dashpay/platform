const rewiremock = require('rewiremock/node');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDapContractError = require('../../../lib/dapContract/errors/InvalidDapContractError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('DapContractFactory', () => {
  let DapContractFactory;
  let decodeMock;
  let validateDapContractMock;
  let createDapContractMock;
  let factory;
  let dapContract;
  let rawDapContract;

  beforeEach(function beforeEach() {
    dapContract = getDapContractFixture();
    rawDapContract = dapContract.toJSON();

    decodeMock = this.sinonSandbox.stub();
    validateDapContractMock = this.sinonSandbox.stub();
    createDapContractMock = this.sinonSandbox.stub().returns(dapContract);

    // Require Factory module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/dapContract/DapContractFactory');

    DapContractFactory = rewiremock.proxy('../../../lib/dapContract/DapContractFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
    });

    factory = new DapContractFactory(
      createDapContractMock,
      validateDapContractMock,
    );
  });

  describe('create', () => {
    it('should return new DapContract with specified name and objects definition', () => {
      const result = factory.create(
        rawDapContract.name,
        rawDapContract.dapObjectsDefinition,
      );

      expect(result).to.be.equal(dapContract);

      expect(createDapContractMock).to.be.calledOnceWith({
        name: rawDapContract.name,
        dapObjectsDefinition: rawDapContract.dapObjectsDefinition,
      });
    });
  });

  describe('createFromObject', () => {
    it('should return new DapContract with data from passed object', () => {
      validateDapContractMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDapContract);

      expect(result).to.be.equal(dapContract);

      expect(validateDapContractMock).to.be.calledOnceWith(rawDapContract);

      expect(createDapContractMock).to.be.calledOnceWith(rawDapContract);
    });

    it('should throw error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDapContractMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDapContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDapContractError);
      expect(error.getRawDapContract()).to.be.equal(rawDapContract);

      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.be.equal(validationError);

      expect(validateDapContractMock).to.be.calledOnceWith(rawDapContract);

      expect(createDapContractMock).not.to.be.called();
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new DapContract from serialized DapContract', () => {
      const serializedDapContract = dapContract.serialize();

      decodeMock.returns(rawDapContract);

      factory.createFromObject.returns(dapContract);

      const result = factory.createFromSerialized(serializedDapContract);

      expect(result).to.be.equal(dapContract);

      expect(factory.createFromObject).to.be.calledOnceWith(rawDapContract);

      expect(decodeMock).to.be.calledOnceWith(serializedDapContract);
    });
  });
});
