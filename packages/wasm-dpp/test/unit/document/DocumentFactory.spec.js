const bs58 = require('bs58');

const Document = require('../../../lib/document/Document');

const DocumentCreateTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition/documentTransition/DocumentCreateTransition');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const Identifier = require('../../../lib/identifier/Identifier');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');
const InvalidDocumentError = require('../../../lib/document/errors/InvalidDocumentError');
const InvalidActionNameError = require('../../../lib/document/errors/InvalidActionNameError');
const NoDocumentsSuppliedError = require('../../../lib/document/errors/NoDocumentsSuppliedError');
const MismatchOwnerIdsError = require('../../../lib/document/errors/MismatchOwnerIdsError');
const InvalidInitialRevisionError = require('../../../lib/document/errors/InvalidInitialRevisionError');
const SerializedObjectParsingError = require('../../../lib/errors/consensus/basic/decode/SerializedObjectParsingError');

const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');
const createDPPMock = require('../../../lib/test/mocks/createDPPMock');
const SomeConsensusError = require('../../../lib/test/mocks/SomeConsensusError');
const entropyGenerator = require('../../../lib/util/entropyGenerator');
const DocumentFactory = require('../../../lib/document/DocumentFactory');

describe('DocumentFactory', () => {
  let decodeProtocolEntityMock;
  let generateEntropyMock;
  let validateDocumentMock;
  let fetchAndValidateDataContractMock;
  let ownerId;
  let dataContract;
  let document;
  let documents;
  let rawDocument;
  let factory;
  let fakeTime;
  let fakeTimeDate;
  let entropy;
  let dppMock;

  beforeEach(function beforeEach() {
    ({ ownerId } = getDocumentsFixture);
    dataContract = getDataContractFixture();

    documents = getDocumentsFixture(dataContract);
    ([,,, document] = documents);
    rawDocument = document.toObject();

    decodeProtocolEntityMock = this.sinonSandbox.stub();
    generateEntropyMock = this.sinonSandbox.stub(entropyGenerator, 'generate');
    validateDocumentMock = this.sinonSandbox.stub();

    validateDocumentMock.returns(new ValidationResult());

    entropy = bs58.decode('789');

    generateEntropyMock.returns(entropy);

    const fetchContractResult = new ValidationResult();
    fetchContractResult.setData(dataContract);

    fetchAndValidateDataContractMock = this.sinonSandbox.stub().returns(fetchContractResult);

    dppMock = createDPPMock();

    factory = new DocumentFactory(
      dppMock,
      validateDocumentMock,
      fetchAndValidateDataContractMock,
      decodeProtocolEntityMock,
    );

    fakeTimeDate = new Date();
    fakeTime = this.sinonSandbox.useFakeTimers(fakeTimeDate);
  });

  afterEach(() => {
    fakeTime.reset();
    generateEntropyMock.restore();
  });

  describe('create', () => {
    it('should return new Document with specified type and data', () => {
      const contractId = bs58.decode('FQco85WbwNgb5ix8QQAH6wurMcgEC5ENSCv5ixG9cj12');
      const name = 'Cutie';

      ownerId = bs58.decode('5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq');
      dataContract.id = Identifier.from(contractId);

      const newDocument = factory.create(
        dataContract,
        ownerId,
        rawDocument.$type,
        { name },
      );

      expect(newDocument).to.be.an.instanceOf(Document);

      expect(newDocument.getType()).to.equal(rawDocument.$type);

      expect(newDocument.get('name')).to.equal(name);

      expect(newDocument.getDataContractId().toBuffer()).to.deep.equal(contractId);
      expect(newDocument.getOwnerId().toBuffer()).to.deep.equal(ownerId);

      expect(generateEntropyMock).to.have.been.calledOnce();
      expect(newDocument.getEntropy()).to.deep.equal(entropy);

      expect(newDocument.getRevision()).to.equal(DocumentCreateTransition.INITIAL_REVISION);

      expect(newDocument.getId()).to.deep.equal(bs58.decode('B99gjrjq6R1FXwGUQnoP7VrmCDDT1PbKprUNzjVbxXfz'));

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

    it('should throw an error if validation faled', () => {
      const error = new Error('validation failed');
      const validationResult = new ValidationResult();
      validationResult.addError(error);

      validateDocumentMock.returns(validationResult);

      try {
        factory.create(dataContract, ownerId, rawDocument.$type);

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);
      }
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      validateDocumentMock.returns(new ValidationResult());

      const result = await factory.createFromObject(rawDocument);

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toObject()).to.deep.equal(document.toObject());

      expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWithExactly(rawDocument);

      expect(validateDocumentMock).to.have.been.calledOnceWithExactly(
        rawDocument, dataContract,
      );
    });

    it('should return new Document without validation if "skipValidation" option is passed', async function it() {
      const resultMock = {
        isValid: () => true,
        merge: this.sinonSandbox.stub(),
        getData: () => getDataContractFixture(),
      };

      fetchAndValidateDataContractMock.resolves(resultMock);

      const result = await factory.createFromObject(rawDocument, { skipValidation: true });

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toObject()).to.deep.equal(document.toObject());

      expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWithExactly(rawDocument);
      expect(validateDocumentMock).to.have.not.been.called();
      expect(resultMock.merge).to.have.not.been.called();
    });

    it('should throw InvalidDocumentError if passed object is not valid', async () => {
      const validationError = new SomeConsensusError('test');

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

        expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWithExactly(rawDocument);
        expect(validateDocumentMock).to.have.been.calledOnceWithExactly(rawDocument, dataContract);
      }
    });

    it('should throw InvalidDocumentError if Data Contract is not valid', async () => {
      const fetchContractError = new SomeConsensusError('error');

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

  describe('createFromBuffer', () => {
    let serializedDocument;

    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factory, 'createFromObject');
      // eslint-disable-next-line prefer-destructuring
      document = documents[8]; // document with binary fields

      serializedDocument = document.toBuffer();
      rawDocument = document.toObject();
    });

    afterEach(() => {
      factory.createFromObject.restore();
    });

    it('should return new Document from serialized one', async () => {
      decodeProtocolEntityMock.returns([rawDocument.$protocolVersion, rawDocument]);

      factory.createFromObject.returns(document);

      const result = await factory.createFromBuffer(serializedDocument);

      expect(result).to.equal(document);

      expect(factory.createFromObject).to.have.been.calledOnceWith(rawDocument);

      expect(decodeProtocolEntityMock).to.have.been.calledOnceWith(
        serializedDocument,
      );
    });

    it('should throw InvalidDocumentError if the decoding fails with consensus error', async () => {
      const parsingError = new SerializedObjectParsingError(
        serializedDocument,
        new Error(),
      );

      decodeProtocolEntityMock.throws(parsingError);

      try {
        await factory.createFromBuffer(serializedDocument);

        expect.fail('should throw InvalidDocumentError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);

        const [innerError] = e.getErrors();
        expect(innerError).to.equal(parsingError);
      }
    });

    it('should throw an error if decoding fails with any other error', async () => {
      const parsingError = new Error('Something failed during parsing');

      decodeProtocolEntityMock.throws(parsingError);

      try {
        await factory.createFromBuffer(serializedDocument);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.equal(parsingError);
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
      documents[0].ownerId = generateRandomIdentifier().toBuffer();
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
      const [newDocument] = getDocumentsFixture(dataContract);

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
