const rewiremock = require('rewiremock/node');

const Document = require('../../../lib/document/Document');
const DocumentsStateTransition = require('../../../lib/document/stateTransition/DocumentsStateTransition');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');
const InvalidDocumentError = require('../../../lib/document/errors/InvalidDocumentError');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const SerializedObjectParsingError = require('../../../lib/errors/SerializedObjectParsingError');

describe('DocumentFactory', () => {
  let decodeMock;
  let generateMock;
  let validateDocumentMock;
  let fetchAndValidateDataContractMock;
  let DocumentFactory;
  let userId;
  let dataContract;
  let document;
  let documents;
  let rawDocument;
  let factory;

  beforeEach(function beforeEach() {
    ({ userId } = getDocumentsFixture);
    dataContract = getDataContractFixture();

    documents = getDocumentsFixture();
    ([document] = documents);
    rawDocument = document.toJSON();

    decodeMock = this.sinonSandbox.stub();
    generateMock = this.sinonSandbox.stub();
    validateDocumentMock = this.sinonSandbox.stub();

    const fetchContractResult = new ValidationResult();
    fetchContractResult.setData(dataContract);

    fetchAndValidateDataContractMock = this.sinonSandbox.stub().returns(fetchContractResult);

    DocumentFactory = rewiremock.proxy('../../../lib/document/DocumentFactory', {
      '../../../lib/util/serializer': { decode: decodeMock },
      '../../../lib/util/entropy': { generate: generateMock },
      '../../../lib/document/Document': Document,
      '../../../lib/document/stateTransition/DocumentsStateTransition': DocumentsStateTransition,
    });

    factory = new DocumentFactory(
      validateDocumentMock,
      fetchAndValidateDataContractMock,
    );
  });

  describe('create', () => {
    it('should return new Document with specified type and data', () => {
      const contractId = dataContract.getId();
      const entropy = '789';
      const name = 'Cutie';

      generateMock.returns(entropy);

      const newDocument = factory.create(
        dataContract,
        userId,
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

      try {
        factory.create(dataContract, userId, type);

        expect.fail('InvalidDocumentTypeError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentTypeError);
        expect(e.getType()).to.equal(type);
        expect(e.getDataContract()).to.equal(dataContract);
      }
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      validateDocumentMock.returns(new ValidationResult());

      const result = await factory.createFromObject(rawDocument);

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toJSON()).to.deep.equal(rawDocument);

      expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWith(rawDocument);

      expect(validateDocumentMock).to.have.been.calledOnceWith(
        rawDocument,
        dataContract,
        { skipValidation: false },
      );
    });

    it('should return new Document without validation if "skipValidation" option is passed', async () => {
      const result = await factory.createFromObject(rawDocument, { skipValidation: true });

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toJSON()).to.deep.equal(rawDocument);

      expect(fetchAndValidateDataContractMock).to.have.not.been.called();
      expect(validateDocumentMock).to.have.not.been.called();
    });

    it('should throw InvalidDocumentError if passed object is not valid', async () => {
      const validationError = new ConsensusError('test');

      validateDocumentMock.returns(
        new ValidationResult([validationError]),
      );

      try {
        await factory.createFromObject(rawDocument);

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);

        expect(e.getErrors()).to.have.length(1);
        expect(e.getRawDocument()).to.equal(rawDocument);

        const [consensusError] = e.getErrors();
        expect(consensusError).to.equal(validationError);

        expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWith(rawDocument);
        expect(validateDocumentMock).to.have.been.calledOnceWith(rawDocument, dataContract);
      }
    });

    it('should throw InvalidDocumentError if Data Contract is not valid', async () => {
      const fetchContractError = new ConsensusError('error');

      fetchAndValidateDataContractMock.returns(
        new ValidationResult([fetchContractError]),
      );

      try {
        await factory.createFromObject(rawDocument);

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);

        expect(e.getErrors()).to.have.length(1);
        expect(e.getRawDocument()).to.equal(rawDocument);

        const [consensusError] = e.getErrors();

        expect(consensusError).to.equal(fetchContractError);

        expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWith(rawDocument);
        expect(validateDocumentMock).to.have.not.been.called();
      }
    });
  });

  describe('createFromSerialized', () => {
    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
    });

    it('should return new Data Contract from serialized Contract', async () => {
      const serializedDocument = document.serialize();

      decodeMock.returns(rawDocument);

      factory.createFromObject.returns(document);

      const result = await factory.createFromSerialized(serializedDocument);

      expect(result).to.equal(document);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDocument);

      expect(decodeMock).to.have.been.calledOnceWith(serializedDocument);
    });

    it('should throw consensus error if `decode` fails', async () => {
      const parsingError = new Error('Something failed during parsing');

      const serializedDocument = document.serialize();

      decodeMock.throws(parsingError);

      try {
        await factory.createFromSerialized(serializedDocument);
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);

        const [innerError] = e.getErrors();

        expect(innerError).to.be.an.instanceOf(SerializedObjectParsingError);
        expect(innerError.getPayload()).to.deep.equal(serializedDocument);
        expect(innerError.getParsingError()).to.deep.equal(parsingError);
      }
    });
  });

  describe('createStateTransition', () => {
    it('should create DocumentsStateTransition with passed documents', () => {
      const result = factory.createStateTransition(documents);

      expect(result).to.be.instanceOf(DocumentsStateTransition);
      expect(result.getDocuments()).to.equal(documents);
    });
  });
});
