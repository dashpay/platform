const rewiremock = require('rewiremock/node');

const Document = require('../../../lib/document/Document');
const DocumentsBatchTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition');

const DocumentCreateTransition = require('../../../lib/document/stateTransition/documentTransition/DocumentCreateTransition');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');
const InvalidDocumentError = require('../../../lib/document/errors/InvalidDocumentError');
const InvalidActionNameError = require('../../../lib/document/errors/InvalidActionNameError');
const NoDocumentsSuppliedError = require('../../../lib/document/errors/NoDocumentsSuppliedError');
const MismatchOwnerIdsError = require('../../../lib/document/errors/MismatchOwnerIdsError');
const InvalidInitialRevisionError = require('../../../lib/document/errors/InvalidInitialRevisionError');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const SerializedObjectParsingError = require('../../../lib/errors/SerializedObjectParsingError');

const generateRandomId = require('../../../lib/test/utils/generateRandomId');

describe('DocumentFactory', () => {
  let decodeMock;
  let generateMock;
  let validateDocumentMock;
  let fetchAndValidateDataContractMock;
  let DocumentFactory;
  let ownerId;
  let dataContract;
  let document;
  let documents;
  let rawDocument;
  let factory;
  let fakeTime;
  let fakeTimeDate;

  beforeEach(function beforeEach() {
    ({ ownerId } = getDocumentsFixture);
    dataContract = getDataContractFixture();

    documents = getDocumentsFixture();
    ([,,, document] = documents);
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
      '../../../lib/document/stateTransition/DocumentsBatchTransition': DocumentsBatchTransition,
    });

    factory = new DocumentFactory(
      validateDocumentMock,
      fetchAndValidateDataContractMock,
    );

    fakeTimeDate = new Date();
    fakeTime = this.sinonSandbox.useFakeTimers(fakeTimeDate);
  });

  afterEach(() => {
    fakeTime.reset();
  });

  describe('create', () => {
    it('should return new Document with specified type and data', () => {
      const contractId = 'FQco85WbwNgb5ix8QQAH6wurMcgEC5ENSCv5ixG9cj12';
      const entropy = '789';
      const name = 'Cutie';

      ownerId = '5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq';
      dataContract.id = contractId;

      generateMock.returns(entropy);

      const newDocument = factory.create(
        dataContract,
        ownerId,
        rawDocument.$type,
        { name },
      );

      expect(newDocument).to.be.an.instanceOf(Document);

      expect(newDocument.getType()).to.equal(rawDocument.$type);

      expect(newDocument.get('name')).to.equal(name);

      expect(newDocument.getDataContractId()).to.equal(contractId);
      expect(newDocument.getOwnerId()).to.equal(ownerId);

      expect(generateMock).to.have.been.calledOnce();
      expect(newDocument.getEntropy()).to.equal(entropy);

      expect(newDocument.getRevision()).to.equal(DocumentCreateTransition.INITIAL_REVISION);

      expect(newDocument.getId()).to.equal('B99gjrjq6R1FXwGUQnoP7VrmCDDT1PbKprUNzjVbxXfz');

      expect(newDocument.getCreatedAt().getTime()).to.be.equal(fakeTimeDate.getTime());
      expect(newDocument.getCreatedAt().getTime()).to.equal(newDocument.getUpdatedAt().getTime());
    });

    it('should throw an error if type is not defined', () => {
      const type = 'wrong';

      try {
        factory.create(dataContract, ownerId, type);

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
    it('should throw and error if documents have unknown action', () => {
      try {
        factory.createStateTransition({
          unknown: documents,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidActionNameError);
        expect(e.getActions()).to.have.deep.members(['unknown']);
      }
    });

    it('should throw and error if no documents were supplied', () => {
      try {
        factory.createStateTransition({});
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(NoDocumentsSuppliedError);
      }
    });

    it('should throw and error if documents have mixed owner ids', () => {
      documents[0].ownerId = generateRandomId();
      try {
        factory.createStateTransition({
          create: documents,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MismatchOwnerIdsError);
        expect(e.getDocuments()).to.have.deep.members(documents);
      }
    });

    it('should throw and error if create documents have invalid initial version', () => {
      documents[0].setRevision(3);
      try {
        factory.createStateTransition({
          create: documents,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidInitialRevisionError);
        expect(e.getDocument()).to.deep.equal(documents[0]);
      }
    });

    it('should create DocumentsBatchTransition with passed documents', () => {
      const [newDocument] = getDocumentsFixture();

      fakeTime.tick(1000);

      const stateTransition = factory.createStateTransition({
        create: documents,
        replace: [newDocument],
      });

      const expectedTransitions = getDocumentTransitionsFixture({
        create: documents,
        replace: [newDocument],
      });

      expectedTransitions.slice(-1).updatedAt = new Date();

      expect(stateTransition.getTransitions()).to.deep.equal(
        expectedTransitions,
      );
    });
  });
});
