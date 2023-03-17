const bs58 = require('bs58');
const DocumentJs = require('@dashevo/dpp/lib/document/Document');
const DocumentCreateTransition = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/DocumentCreateTransition');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');
const IdentifierJs = require('@dashevo/dpp/lib/identifier/Identifier');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const entropyGenerator = require('@dashevo/dpp/lib/util/entropyGenerator');
const DocumentFactoryJS = require('@dashevo/dpp/lib/document/DocumentFactory');

let {
  Identifier, DocumentFactory, DataContract, Document, DocumentValidator, ProtocolVersionValidator,
  InvalidDocumentTypeInDataContractError, InvalidDocumentError, JsonSchemaError,
  NoDocumentsSuppliedError, MismatchOwnerIdsError, InvalidInitialRevisionError,
  InvalidActionNameError,
} = require('../../..');

const { default: loadWasmDpp } = require('../../..');

describe('DocumentFactory', () => {
  let decodeProtocolEntityMock;
  let generateEntropyMock;
  let validateDocumentMock;
  let fetchAndValidateDataContractMock;
  let stateRepositoryMock;
  let ownerIdJs;
  let ownerId;
  let dataContract;
  let dataContractJs;
  let document;
  let documentJs;
  let documentsJs;
  let documents;
  let rawDocument;
  let rawDocumentJs;
  let factoryJs;
  let factory;
  let fakeTime;
  let fakeTimeDate;
  let entropy;
  let dppMock;
  let dataContractId;
  let documentValidator;

  beforeEach(async () => {
    ({
      Identifier, ProtocolVersionValidator, DocumentValidator, DocumentFactory,
      DataContract, Document,
      // Errors:
      InvalidDocumentTypeInDataContractError,
      InvalidDocumentError,
      JsonSchemaError,
      NoDocumentsSuppliedError,
      MismatchOwnerIdsError,
      InvalidInitialRevisionError,
      InvalidActionNameError,
    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {
    const protocolValidator = new ProtocolVersionValidator();
    documentValidator = new DocumentValidator(protocolValidator);

    ({ ownerId: ownerIdJs } = getDocumentsFixture);
    ownerId = Identifier.from(ownerIdJs);

    dataContractJs = getDataContractFixture();
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());
    const dc = DataContract.fromBuffer(dataContractJs.toBuffer());
    dataContractId = dataContractJs.getId().toBuffer();

    documentsJs = getDocumentsFixture(dataContractJs);
    documents = documentsJs.map((d) => {
      const doc = new Document(d.toObject(), dataContract);
      doc.setEntropy(d.entropy);
      return doc;
    });

    ([, , , documentJs] = documentsJs);
    rawDocumentJs = documentJs.toObject();
    ([, , , document] = documents);
    rawDocument = document.toObject();

    decodeProtocolEntityMock = this.sinonSandbox.stub();
    generateEntropyMock = this.sinonSandbox.stub(entropyGenerator, 'generate');
    validateDocumentMock = this.sinonSandbox.stub();

    validateDocumentMock.returns(new ValidationResult());
    entropy = bs58.decode('789');
    generateEntropyMock.returns(entropy);

    const fetchContractResult = new ValidationResult();
    fetchContractResult.setData(dataContractJs);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dc);

    fetchAndValidateDataContractMock = this.sinonSandbox.stub().returns(fetchContractResult);
    dppMock = createDPPMock();

    factory = new DocumentFactory(1, documentValidator, stateRepositoryMock);
    factoryJs = new DocumentFactoryJS(
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
      // NiceDocument is used instead of IndexedDocument, because the NiceDocument
      // should be valid for this test and it passes the DataContract validation.
      // The previous version of test passed due to the fact that
      // the mocked DataContract validator always returned true.
      const [niceDocument] = documentsJs;

      const newRawDocument = niceDocument.toObject();
      const contractId = bs58.decode('FQco85WbwNgb5ix8QQAH6wurMcgEC5ENSCv5ixG9cj12');
      const name = 'Cutie';

      ownerIdJs = bs58.decode('5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq');
      ownerId = Identifier.from(ownerIdJs);

      dataContractJs.id = IdentifierJs.from(contractId);
      dataContract.setId(Identifier.from(contractId));

      const newDocumentJs = factoryJs.create(
        dataContractJs,
        ownerIdJs,
        newRawDocument.$type,
        { name },
      );

      const newDocument = factory.create(
        dataContract,
        ownerIdJs,
        newRawDocument.$type,
        { name },
      );

      expect(newDocument).to.be.an.instanceOf(Document);
      expect(newDocumentJs).to.be.an.instanceOf(DocumentJs);

      expect(newDocumentJs.getType()).to.equal(newRawDocument.$type);
      expect(newDocument.getType()).to.equal(newRawDocument.$type);

      expect(newDocumentJs.get('name')).to.equal(name);
      expect(newDocument.get('name')).to.equal(name);

      expect(newDocumentJs.getDataContractId().toBuffer()).to.deep.equal(contractId);
      expect(newDocument.getDataContractId().toBuffer()).to.deep.equal(contractId);

      expect(newDocumentJs.getOwnerId().toBuffer()).to.deep.equal(ownerIdJs);
      expect(newDocument.getOwnerId().toBuffer()).to.deep.equal(ownerIdJs);

      expect(generateEntropyMock).to.have.been.calledOnce();
      expect(newDocumentJs.getEntropy()).to.deep.equal(entropy);

      expect(newDocumentJs.getRevision()).to.equal(DocumentCreateTransition.INITIAL_REVISION);
      expect(newDocument.getRevision()).to.equal(DocumentCreateTransition.INITIAL_REVISION);

      expect(newDocumentJs.getId()).to.deep.equal(bs58.decode('E9QpjZMD7CPAGa7x2ABuLFPvBLZjhPji4TMrUfSP3Hk9'));
      // in case of rust version, it is impossible to test the ID, because the ID
      // is generated based on entropy generator which generates different output
      // every time and it cannot be mocked. ID generation should be verified
      // in a true unit test. Not here.
      expect(newDocument.getEntropy()).not.to.deep.be.equal(Buffer.alloc(32));

      expect(newDocumentJs.getCreatedAt().getTime()).to.be.equal(fakeTimeDate.getTime());
      expect(newDocument.getCreatedAt()).to.be.an('number');
    });

    it('should throw an error if type is not defined', () => {
      const type = 'wrong';

      try {
        factory.create(dataContract, ownerId, type, {});

        expect.fail('InvalidDocumentTypeError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentTypeInDataContractError);
        expect(e.getType()).to.equal(type);
        expect(e.getDataContractId().toBuffer()).to.deep.equal(dataContractId);
      }
    });

    it('should throw an error if validation failed', () => {
      const error = new Error('validation failed');
      const validationResult = new ValidationResult();
      validationResult.addError(error);

      validateDocumentMock.returns(validationResult);

      try {
        factory.create(dataContract, ownerId, rawDocumentJs.$type, {});

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);
      }
    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      const result = await factory.createFromObject(rawDocument);

      expect(result).to.be.an.instanceOf(Document);
      expect(result.toObject()).to.deep.equal(document.toObject());

      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    });

    it('should return new Document without validation if "skipValidation" option is passed - Rust', async () => {
      delete rawDocument.lastName;
      const result = await factory.createFromObject(rawDocument, { skipValidation: true });
      expect(result).to.be.an.instanceOf(Document);

      expect(result.toObject()).to.deep.equal(rawDocument);
      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    });

    it('should throw InvalidDocumentError if passed object is not valid', async () => {
      delete rawDocument.lastName;

      try {
        await factory.createFromObject(rawDocument);

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);
        expect(e.getErrors()).to.have.length(1);

        // Identifiers don't survive conversion back and forth unless done through
        // a Document constructor
        expect(
          (new Document(e.getRawDocument(), dataContract)).toObject(),
        ).to.deep.equal(rawDocument);

        const [consensusError] = e.getErrors();
        expect(consensusError).to.be.an.instanceOf(JsonSchemaError);
      }
    });

    it('should throw InvalidDocumentError if Data Contract is not valid', async () => {
      const dc = DataContract.fromBuffer(dataContractJs.toBuffer());
      dc.setDocuments({ '$%34': { '^&*': 'Keck' } });
      const oldDataContract = DataContract.fromBuffer(dataContractJs.toBuffer());
      stateRepositoryMock.fetchDataContract.resolves(dc);

      try {
        await factory.createFromObject(rawDocumentJs);

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);

        expect(e.getErrors()).to.have.length(1);
        expect(
          (new Document(e.getRawDocument(), oldDataContract).toObject()),
        ).to.deep.equal(rawDocumentJs);

        expect(stateRepositoryMock.fetchDataContract.callCount).to.be.equal(1);
        const callArguments = stateRepositoryMock.fetchDataContract.getCall(0).args[0];

        // Due to wasm-bindgen boundry we need to do some additional work to retrieve the exact
        // Buffer it has been called with
        expect(Buffer.from(callArguments)).to.be.deep.equal(dc.getId().toBuffer());
      }
    });
  });

  describe('createFromBuffer', () => {
    let serializedDocument;

    beforeEach(function beforeEach() {
      this.sinonSandbox.stub(factoryJs, 'createFromObject');
      // eslint-disable-next-line prefer-destructuring
      documentJs = documentsJs[8]; // document with binary fields

      serializedDocument = documentJs.toBuffer();
      rawDocumentJs = documentJs.toObject();
    });

    afterEach(() => {
      factoryJs.createFromObject.restore();
    });

    it('should return new Document from serialized one', async () => {
      const result = await factory.createFromBuffer(serializedDocument);
      expect(result.toObject()).to.deep.equal(documentJs.toObject());
      expect(stateRepositoryMock.fetchDataContract).to.have.been.calledOnce();
    });

    it('should throw InvalidDocumentError if the decoding fails with consensus error', async () => {
      documentJs.data = 'Not a valid data';
      serializedDocument = documentJs.toBuffer();

      try {
        await factory.createFromBuffer(serializedDocument);

        expect.fail('should throw InvalidDocumentError');
      } catch (e) {
        console.log(e);
        expect(e).to.be.an.instanceOf(InvalidDocumentError);
      }
    });

    it('should throw an error if decoding fails with any other error - Rust', async () => {
      const serializeDocument = Buffer.alloc(160, 1);
      try {
        await factory.createFromBuffer(serializeDocument);

        expect.fail('should throw an error');
      } catch (e) {
        console.log(e.toString());
        // TODO - parsing errors are not handled yet, as they happen directly in the rust code when
        //  trying to access a field
        expect(e).to.startsWith('Error conversion not implemented:');
      }
    });
  }); describe('createStateTransition', () => {
    it('should throw and error if documents have unknown action', async () => {
      try {
        factory.createStateTransition({
          unknown: documents,
        });

        expect.fail('Error was not thrown');
      } catch (e) {
        // documents of unknown actions are filtered out
        expect(e).to.be.an.instanceOf(InvalidActionNameError);
        expect(e.getActions()).to.have.deep.members(['unknown']);
      }
    });

    it('should throw and error if no documents were supplied', async () => {
      try {
        factory.createStateTransition({});
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(NoDocumentsSuppliedError);
      }
    });

    it('should throw and error if documents have mixed owner ids', async () => {
      const newId = generateRandomIdentifier().toBuffer();
      documents[0].setOwnerId(new Identifier(newId));
      const rawDocuments = documents.map((d) => d.toObject());

      try {
        factory.createStateTransition({
          create: documents,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MismatchOwnerIdsError);
        const rawDocumentsFromError = e.getDocuments().map((d) => d.toObject());
        expect(rawDocumentsFromError).to.have.deep.members(rawDocuments);
      }
    });

    it('should throw and error if create documents have invalid initial version', async () => {
      documents[0].setRevision(3);
      const expectedDocument = documents[0].toObject();
      try {
        factory.createStateTransition({
          create: documents,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidInitialRevisionError);
        expect(e.getDocument().toObject()).to.deep.equal(expectedDocument);
      }
    });

    it('should create DocumentsBatchTransition with passed documents', async () => {
      const [newDocumentJs] = getDocumentsFixture(dataContractJs);
      const newDocument = new Document(newDocumentJs.toObject(), dataContract);

      const stateTransition = factory.createStateTransition({
        create: documents,
        replace: [newDocument],
      });

      const stateTransitionJs = factoryJs.createStateTransition({
        create: documentsJs,
        replace: [newDocumentJs],
      });

      const transitions = stateTransition.getTransitions().map((t) => t.toObject());
      const expectedTransitions = stateTransitionJs.getTransitions().map((t) => t.toObject());

      expect(transitions).to.deep.includes.members(expectedTransitions);
    });
  });
});
