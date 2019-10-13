const rewiremock = require('rewiremock/node');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDataContractError = require('../../../lib/dataContract/errors/InvalidDataContractError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('DataContractFactory', () => {
  let DataContractFactory;
  let decodeMock;
  let validateDataContractMock;
  let createDataContractMock;
  let factory;
  let dataContract;
  let rawDataContract;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toJSON();

    decodeMock = this.sinonSandbox.stub();
    validateDataContractMock = this.sinonSandbox.stub();
    createDataContractMock = this.sinonSandbox.stub().returns(dataContract);

    // Require Factory module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/dataContract/DataContractFactory');

    DataContractFactory = rewiremock.proxy('../../../lib/dataContract/DataContractFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
    });

    factory = new DataContractFactory(
      createDataContractMock,
      validateDataContractMock,
    );
  });

  describe('create', () => {
    it('should return new Data Contract with specified name and documents definition', () => {
      const result = factory.create(
        rawDataContract.contractId,
        rawDataContract.documents,
      );

      expect(result).to.equal(dataContract);

      expect(createDataContractMock).to.have.been.calledOnceWith({
        contractId: rawDataContract.contractId,
        documents: rawDataContract.documents,
      });
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', () => {
      validateDataContractMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDataContract);

      expect(result).to.equal(dataContract);

      expect(validateDataContractMock).to.have.been.calledOnceWith(rawDataContract);

      expect(createDataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });

    it('should return new Data Contract without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawDataContract, { skipValidation: true });

      expect(result).to.equal(dataContract);

      expect(validateDataContractMock).to.have.not.been.called();

      expect(createDataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });

    it('should throw an error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDataContractMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDataContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDataContractError);
      expect(error.getRawDataContract()).to.equal(rawDataContract);

      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.equal(validationError);

      expect(validateDataContractMock).to.have.been.calledOnceWith(rawDataContract);

      expect(createDataContractMock).to.have.not.been.called();
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Data Contract from serialized contract', () => {
      const serializedDataContract = dataContract.serialize();

      decodeMock.returns(rawDataContract);

      factory.createFromObject.returns(dataContract);

      const result = factory.createFromSerialized(serializedDataContract);

      expect(result).to.equal(dataContract);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDataContract);

      expect(decodeMock).to.have.been.calledOnceWith(serializedDataContract);
    });
  });
});
