const rewiremock = require('rewiremock/node');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const DataContract = require('../../../lib/dataContract/DataContract');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDataContractError = require('../../../lib/dataContract/errors/InvalidDataContractError');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const SerializedObjectParsingError = require('../../../lib/errors/SerializedObjectParsingError');

describe('DataContractFactory', () => {
  let DataContractFactory;
  let decodeMock;
  let validateDataContractMock;
  let DataContractMock;
  let factory;
  let dataContract;
  let rawDataContract;
  let entropyMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    rawDataContract = dataContract.toJSON();

    decodeMock = this.sinonSandbox.stub();
    validateDataContractMock = this.sinonSandbox.stub();
    DataContractMock = this.sinonSandbox.stub().returns(dataContract);
    DataContractMock.DEFAULTS = DataContract.DEFAULTS;
    entropyMock = {
      generate: this.sinonSandbox.stub(),
    };

    // Require Factory module for webpack
    // eslint-disable-next-line global-require
    require('../../../lib/dataContract/DataContractFactory');

    DataContractFactory = rewiremock.proxy('../../../lib/dataContract/DataContractFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/util/entropy': entropyMock,
      '../../../lib/dataContract/stateTransition/DataContractCreateTransition': DataContractCreateTransition,
      '../../../lib/dataContract/DataContract': DataContractMock,
    });

    factory = new DataContractFactory(
      validateDataContractMock,
    );
  });

  describe('create', () => {
    it('should return new Data Contract with specified name and documents definition', () => {
      entropyMock.generate.returns(dataContract.getEntropy());
      const result = factory.create(
        rawDataContract.ownerId,
        rawDataContract.documents,
      );

      expect(result).to.equal(dataContract);

      expect(DataContractMock).to.have.been.calledOnceWith({
        $schema: DataContract.DEFAULTS.SCHEMA,
        $id: rawDataContract.$id,
        ownerId: rawDataContract.ownerId,
        documents: rawDataContract.documents,
        definitions: {},
      });
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      validateDataContractMock.returns(new ValidationResult());

      const result = await factory.createFromObject(rawDataContract);

      expect(result).to.equal(dataContract);

      expect(validateDataContractMock).to.have.been.calledOnceWith(rawDataContract);

      expect(DataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });

    it('should return new Data Contract without validation if "skipValidation" option is passed', async () => {
      const result = await factory.createFromObject(rawDataContract, { skipValidation: true });

      expect(result).to.equal(dataContract);

      expect(validateDataContractMock).to.have.not.been.called();

      expect(DataContractMock).to.have.been.calledOnceWith(rawDataContract);
    });

    it('should throw an error if passed object is not valid', async () => {
      const validationError = new ConsensusError('test');

      validateDataContractMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        await factory.createFromObject(rawDataContract);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDataContractError);
      expect(error.getRawDataContract()).to.equal(rawDataContract);

      expect(error.getErrors()).to.have.length(1);

      const [consensusError] = error.getErrors();

      expect(consensusError).to.equal(validationError);

      expect(validateDataContractMock).to.have.been.calledOnceWith(rawDataContract);

      expect(DataContractMock).to.have.not.been.called();
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Data Contract from serialized contract', async () => {
      const serializedDataContract = dataContract.serialize();

      decodeMock.returns(rawDataContract);

      factory.createFromObject.returns(dataContract);

      const result = await factory.createFromSerialized(serializedDataContract);

      expect(result).to.equal(dataContract);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDataContract);

      expect(decodeMock).to.have.been.calledOnceWith(serializedDataContract);
    });

    it('should throw consensus error if `decode` fails', async () => {
      const parsingError = new Error('Something failed during parsing');

      const serializedDataContract = dataContract.serialize();

      decodeMock.throws(parsingError);

      try {
        await factory.createFromSerialized(serializedDataContract);
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDataContractError);

        const [innerError] = e.getErrors();

        expect(innerError).to.be.an.instanceOf(SerializedObjectParsingError);
        expect(innerError.getPayload()).to.deep.equal(serializedDataContract);
        expect(innerError.getParsingError()).to.deep.equal(parsingError);
      }
    });
  });

  describe('createStateTransition', () => {
    it('should return new DataContractCreateTransition with passed DataContract', () => {
      const result = factory.createStateTransition(dataContract);

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);
      expect(result.getDataContract().toJSON()).to.deep.equal(dataContract.toJSON());
    });
  });
});
