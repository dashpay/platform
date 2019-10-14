const rewiremock = require('rewiremock/node');

const Document = require('../../../lib/document/Document');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');
const InvalidDocumentError = require('../../../lib/document/errors/InvalidDocumentError');
const ConsensusError = require('../../../lib/errors/ConsensusError');

describe('DocumentFactory', () => {
  let decodeMock;
  let generateMock;
  let validateDocumentMock;
  let DocumentFactory;
  let userId;
  let dataContract;
  let document;
  let rawDocument;
  let factory;

  beforeEach(function beforeEach() {
    decodeMock = this.sinonSandbox.stub();
    generateMock = this.sinonSandbox.stub();
    validateDocumentMock = this.sinonSandbox.stub();

    DocumentFactory = rewiremock.proxy('../../../lib/document/DocumentFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/util/entropy': { generate: generateMock },
      '../../../lib/document/Document': Document,
    });

    ({ userId } = getDocumentsFixture);
    dataContract = getDataContractFixture();

    [document] = getDocumentsFixture();
    rawDocument = document.toJSON();

    factory = new DocumentFactory(
      userId,
      dataContract,
      validateDocumentMock,
    );
  });

  describe('create', () => {
    it('should return new Document with specified type and data', () => {
      const contractId = dataContract.getId();
      const entropy = '789';
      const name = 'Cutie';

      generateMock.returns(entropy);

      const newDocument = factory.create(
        rawDocument.$type,
        { name },
      );

      expect(newDocument).to.be.an.instanceOf(Document);

      expect(newDocument.getType()).to.equal(rawDocument.$type);

      expect(newDocument.get('name')).to.equal(name);

      expect(newDocument.contractId).to.equal(contractId);
      expect(newDocument.userId).to.equal(userId);

      expect(generateMock).to.have.been.calledOnce();
      expect(newDocument.entropy).to.equal(entropy);

      expect(newDocument.getAction()).to.equal(Document.DEFAULTS.ACTION);

      expect(newDocument.getRevision()).to.equal(Document.DEFAULTS.REVISION);
    });

    it('should throw an error if type is not defined', () => {
      const type = 'wrong';

      let error;
      try {
        factory.create(type);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
      expect(error.getType()).to.equal(type);
      expect(error.getDataContract()).to.equal(dataContract);
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', () => {
      validateDocumentMock.returns(new ValidationResult());

      const result = factory.createFromObject(rawDocument);

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toJSON()).to.deep.equal(rawDocument);

      expect(validateDocumentMock).to.have.been.calledOnceWith(rawDocument, dataContract);
    });

    it('should return new Document without validation if "skipValidation" option is passed', () => {
      const result = factory.createFromObject(rawDocument, { skipValidation: true });

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toJSON()).to.deep.equal(rawDocument);

      expect(validateDocumentMock).to.have.not.been.called();
    });

    it('should throw an error if passed object is not valid', () => {
      const validationError = new ConsensusError('test');

      validateDocumentMock.returns(new ValidationResult([validationError]));

      let error;
      try {
        factory.createFromObject(rawDocument);
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentError);

      expect(error.getErrors()).to.have.length(1);
      expect(error.getRawDocument()).to.equal(rawDocument);

      const [consensusError] = error.getErrors();
      expect(consensusError).to.equal(validationError);

      expect(validateDocumentMock).to.have.been.calledOnceWith(rawDocument, dataContract);
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Data Contract from serialized Contract', () => {
      const serializedDocument = document.serialize();

      decodeMock.returns(rawDocument);

      factory.createFromObject.returns(document);

      const result = factory.createFromSerialized(serializedDocument);

      expect(result).to.equal(document);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDocument);

      expect(decodeMock).to.have.been.calledOnceWith(serializedDocument);
    });
  });

  describe('setUserId', () => {
    it('should set User ID', () => {
      userId = '123';

      const result = factory.setUserId(userId);

      expect(result).to.equal(factory);
      expect(factory.userId).to.equal(userId);
    });
  });

  describe('getUserId', () => {
    it('should return User ID', () => {
      const result = factory.getUserId();

      expect(result).to.equal(userId);
    });
  });

  describe('setDataContract', () => {
    it('should set Data Contract', () => {
      factory.dataContract = null;

      const result = factory.setDataContract(dataContract);

      expect(result).to.equal(factory);
      expect(factory.dataContract).to.equal(dataContract);
    });
  });

  describe('getDataContract', () => {
    it('should return Data Contract', () => {
      const result = factory.getDataContract();

      expect(result).to.equal(dataContract);
    });
  });
});
