const bs58 = require('bs58');


const DocumentJs = require('@dashevo/dpp/lib/document/Document');

const DocumentCreateTransition = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/DocumentCreateTransition');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const IdentifierJs = require('@dashevo/dpp/lib/identifier/Identifier');

const InvalidDocumentTypeError = require('@dashevo/dpp/lib/errors/InvalidDocumentTypeError');
const InvalidDocumentError = require('@dashevo/dpp/lib/document/errors/InvalidDocumentError');
const InvalidActionNameError = require('@dashevo/dpp/lib/document/errors/InvalidActionNameError');
const NoDocumentsSuppliedError = require('@dashevo/dpp/lib/document/errors/NoDocumentsSuppliedError');
const MismatchOwnerIdsError = require('@dashevo/dpp/lib/document/errors/MismatchOwnerIdsError');
const InvalidInitialRevisionError = require('@dashevo/dpp/lib/document/errors/InvalidInitialRevisionError');
const SerializedObjectParsingError = require('@dashevo/dpp/lib/errors/consensus/basic/decode/SerializedObjectParsingError');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const createDPPMock = require('@dashevo/dpp/lib/test/mocks/createDPPMock');
const SomeConsensusError = require('@dashevo/dpp/lib/test/mocks/SomeConsensusError');
const entropyGenerator = require('@dashevo/dpp/lib/util/entropyGenerator');
const DocumentFactoryJS = require('@dashevo/dpp/lib/document/DocumentFactory');
const { default: loadWasmDpp } = require('../../../dist');

let Identifier;
let DocumentFactory;
let DataContract;
let Document;
let DocumentValidator;
let ProtocolVersionValidator;


describe('DocumentFactory', () => {
  let decodeProtocolEntityMock;
  let generateEntropyMock;
  let validateDocumentMock;
  let fetchAndValidateDataContractMock;
  let ownerIdJs;
  let ownerId;
  let dataContract;
  let dataContractJs;
  let documentJs;
  let documentsJs;
  let documents;
  let rawDocument;
  let factoryJs;
  let factory;
  let fakeTime;
  let fakeTimeDate;
  let entropy;
  let dppMock;




  beforeEach(async () => {
    ({
      Identifier, ProtocolVersionValidator, DocumentValidator, DocumentFactory, DataContract, Document

    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {
    let protocolVersionValidatorRs = new ProtocolVersionValidator();
    let documentValidatorRs = new DocumentValidator(protocolVersionValidatorRs);


    ({ ownerId: ownerIdJs } = getDocumentsFixture);
    ownerId = Identifier.from(ownerIdJs);

    dataContractJs = getDataContractFixture();
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    documentsJs = getDocumentsFixture(dataContractJs);
    documents = documentsJs.map((d) => {
      return new Document(d.toObject(), dataContract)
    });

    ([, , , documentJs] = documentsJs);
    rawDocument = documentJs.toObject();



    decodeProtocolEntityMock = this.sinonSandbox.stub();
    generateEntropyMock = this.sinonSandbox.stub(entropyGenerator, 'generate');
    validateDocumentMock = this.sinonSandbox.stub();

    validateDocumentMock.returns(new ValidationResult());

    entropy = bs58.decode('789');

    generateEntropyMock.returns(entropy);

    const fetchContractResult = new ValidationResult();
    fetchContractResult.setData(dataContractJs);

    fetchAndValidateDataContractMock = this.sinonSandbox.stub().returns(fetchContractResult);

    dppMock = createDPPMock();

    factory = new DocumentFactory(1, documentValidatorRs, {});
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
      // NiceDocument is used instead of IndexedDocument, because the NiceDocument should be valid for this test
      // and it passes the DataContract validation. The previous version of test passed due to the fact that
      // the mocked DataContract validator always returned true.
      const [niceDocument] = documentsJs;

      let rawDocument = niceDocument.toObject();


      const contractId = bs58.decode('FQco85WbwNgb5ix8QQAH6wurMcgEC5ENSCv5ixG9cj12');
      const name = 'Cutie';

      ownerIdJs = bs58.decode('5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq');
      ownerId = Identifier.from(ownerIdJs);

      dataContractJs.id = IdentifierJs.from(contractId);
      dataContract.setId(Identifier.from(contractId));

      const newDocumentJs = factoryJs.create(
        dataContractJs,
        ownerIdJs,
        rawDocument.$type,
        { name },
      );

      const newDocument = factory.create(
        dataContract,
        ownerId,
        rawDocument.$type,
        { name },
      );



      expect(newDocument).to.be.an.instanceOf(Document);
      expect(newDocumentJs).to.be.an.instanceOf(DocumentJs);

      expect(newDocumentJs.getType()).to.equal(rawDocument.$type);
      expect(newDocument.getType()).to.equal(rawDocument.$type);

      expect(newDocumentJs.get('name')).to.equal(name);
      expect(newDocument.get('name')).to.equal(name);

      expect(newDocumentJs.getDataContractId().toBuffer()).to.deep.equal(contractId);
      expect(newDocument.getDataContractId().toBuffer()).to.deep.equal(contractId);

      expect(newDocumentJs.getOwnerId().toBuffer()).to.deep.equal(ownerIdJs);
      expect(newDocument.getOwnerId().toBuffer()).to.deep.equal(ownerIdJs);

      expect(generateEntropyMock).to.have.been.calledOnce();
      expect(newDocumentJs.getEntropy()).to.deep.equal(entropy);
      // we only verify that entropy isn't empty
      expect(newDocument.getEntropy()).not.to.deep.be.equal(Buffer.alloc(32))

      expect(newDocumentJs.getRevision()).to.equal(DocumentCreateTransition.INITIAL_REVISION);
      expect(newDocument.getRevision()).to.equal(DocumentCreateTransition.INITIAL_REVISION);

      expect(newDocumentJs.getId()).to.deep.equal(bs58.decode("E9QpjZMD7CPAGa7x2ABuLFPvBLZjhPji4TMrUfSP3Hk9"));
      // in case of rust version, it is impossible to test the ID, because the ID is generated based on entropy generator
      // which generates different output every time and it cannot be mocked. ID generation should be verified in a true unit test. Not here.
      expect(newDocument.getEntropy()).not.to.deep.be.equal(Buffer.alloc(32))

      expect(newDocumentJs.getCreatedAt().getTime()).to.be.equal(fakeTimeDate.getTime());
      expect(newDocument.getCreatedAt()).to.be.an('number');
    });

    it('should throw an error if type is not defined', () => {
      const type = 'wrong';

      try {
        factoryJs.create(dataContractJs, ownerIdJs, type);

        expect.fail('InvalidDocumentTypeError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentTypeError);
        expect(e.getType()).to.equal(type);
        expect(e.getDataContract()).to.equal(dataContractJs);
      }
    });

    it('should throw an error if type is not defined - Rust', () => {
      const type = 'wrong';

      try {
        factory.create(dataContract, ownerId, type, {});

        expect.fail('InvalidDocumentTypeError should be thrown');
      } catch (e) {

        // TODO - change after error merge
        expect(e).to.contain("Data Contract doesn't define document")
      }
    });


    it('should throw an error if validation failed', () => {
      const error = new Error('validation failed');
      const validationResult = new ValidationResult();
      validationResult.addError(error);

      validateDocumentMock.returns(validationResult);

      try {
        factoryJs.create(dataContractJs, ownerIdJs, rawDocument.$type);

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);
      }
    });

    it('should throw an error if validation failed - Rust', () => {
      const error = new Error('validation failed');
      const validationResult = new ValidationResult();
      validationResult.addError(error);

      validateDocumentMock.returns(validationResult);

      try {
        factory.create(dataContract, ownerId, rawDocument.$type, {});

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {


        // TODO - change when errors merged
        expect(e).to.contain("Invalid Document")
      }

    });
  });

  describe('createFromObject', () => {
    it('should return new Data Contract with data from passed object', async () => {
      validateDocumentMock.returns(new ValidationResult());

      const result = await factoryJs.createFromObject(rawDocument);

      expect(result).to.be.an.instanceOf(DocumentJs);
      expect(result.toObject()).to.deep.equal(documentJs.toObject());

      expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWithExactly(rawDocument);

      expect(validateDocumentMock).to.have.been.calledOnceWithExactly(
        rawDocument, dataContractJs,
      );
    });

    it('should return new Document without validation if "skipValidation" option is passed', async function it() {
      const resultMock = {
        isValid: () => true,
        merge: this.sinonSandbox.stub(),
        getData: () => getDataContractFixture(),
      };

      fetchAndValidateDataContractMock.resolves(resultMock);

      const result = await factoryJs.createFromObject(rawDocument, { skipValidation: true });

      expect(result).to.be.an.instanceOf(DocumentJs);
      expect(result.toObject()).to.deep.equal(documentJs.toObject());

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
        await factoryJs.createFromObject(rawDocument);

        expect.fail('InvalidDocumentError should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentError);

        expect(e.getErrors()).to.have.length(1);
        expect(e.getRawDocument()).to.equal(rawDocument);

        const [consensusError] = e.getErrors();
        expect(consensusError).to.equal(validationError);

        expect(fetchAndValidateDataContractMock).to.have.been.calledOnceWithExactly(rawDocument);
        expect(validateDocumentMock).to.have.been.calledOnceWithExactly(rawDocument, dataContractJs);
      }
    });

    it('should throw InvalidDocumentError if Data Contract is not valid', async () => {
      const fetchContractError = new SomeConsensusError('error');

      fetchAndValidateDataContractMock.returns(
        new ValidationResult([fetchContractError]),
      );

      try {
        await factoryJs.createFromObject(rawDocument);

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
      this.sinonSandbox.stub(factoryJs, 'createFromObject');
      // eslint-disable-next-line prefer-destructuring
      documentJs = documentsJs[8]; // document with binary fields

      serializedDocument = documentJs.toBuffer();
      rawDocument = documentJs.toObject();
    });

    afterEach(() => {
      factoryJs.createFromObject.restore();
    });

    it('should return new Document from serialized one', async () => {
      decodeProtocolEntityMock.returns([rawDocument.$protocolVersion, rawDocument]);

      factoryJs.createFromObject.returns(documentJs);

      const result = await factoryJs.createFromBuffer(serializedDocument);

      expect(result).to.equal(documentJs);

      expect(factoryJs.createFromObject).to.have.been.calledOnceWith(rawDocument);

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
        await factoryJs.createFromBuffer(serializedDocument);

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
        await factoryJs.createFromBuffer(serializedDocument);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.equal(parsingError);
      }
    });
  });

  // TODO <----
  describe('createStateTransition', () => {
    it('should throw and error if documents have unknown action', () => {
      try {
        factoryJs.createStateTransition({
          unknown: documentsJs,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidActionNameError);
        expect(e.getActions()).to.have.deep.members(['unknown']);
      }
    });

    it('should throw and error if documents have unknown action - Rust', () => {
      try {
        factory.createStateTransition({
          unknown: documentsJs,
        }, dataContract);
        expect.fail('Error was not thrown');

      } catch (e) {

        // TODO - change when errors merged
        expect(e).to.contain("unknown action type: 'unknown")
      }
    });

    it('should throw and error if no documents were supplied', () => {
      try {
        factoryJs.createStateTransition({});
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(NoDocumentsSuppliedError);
      }
    });


    it('should throw and error if no documents were supplied - Rust', () => {
      try {
        factory.createStateTransition({}, dataContract);
        expect.fail('Error was not thrown');
      } catch (e) {

        // TODO - change when errors merged
        expect(e).to.contain("ProtocolError: No documents were supplied to state transition")
      }
    });

    it('should throw and error if documents have mixed owner ids', () => {
      documentsJs[0].ownerId = generateRandomIdentifier().toBuffer();
      try {
        factoryJs.createStateTransition({
          create: documentsJs,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MismatchOwnerIdsError);
        expect(e.getDocuments()).to.have.deep.members(documentsJs);
      }
    });

    it('should throw and error if documents have mixed owner ids - Rust', () => {
      const newId = generateRandomIdentifier().toBuffer();
      documents[0].setOwnerId(new Identifier(newId));

      try {
        factory.createStateTransition({
          create: documents,
        }, dataContract);
        expect.fail('Error was not thrown');
      } catch (e) {


        // TODO - change when errors merged
        expect(e).to.contain("ProtocolError: Documents have mixed owner ids")
      }
    });


    it('should throw and error if create documents have invalid initial version', () => {
      documentsJs[0].setRevision(3);
      try {
        factoryJs.createStateTransition({
          create: documentsJs,
        });
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidInitialRevisionError);
        expect(e.getDocument()).to.deep.equal(documentsJs[0]);
      }
    });


    it('should throw and error if create documents have invalid initial version - Rust', () => {
      documents[0].setRevision(3);
      try {
        factory.createStateTransition({
          create: documents,
        }, dataContract);
        expect.fail('Error was not thrown');
      } catch (e) {

        // TODO - change when errors merged
        expect(e).to.contain("ProtocolError: Invalid Document initial revision 3")
      }
    });


    it('should create DocumentsBatchTransition with passed documents', () => {
      const [newDocument] = getDocumentsFixture(dataContractJs);

      fakeTime.tick(1000);

      const stateTransition = factoryJs.createStateTransition({
        create: documentsJs,
        replace: [newDocument],
      });

      const expectedTransitions = getDocumentTransitionsFixture({
        create: documentsJs,
        replace: [newDocument],
      });

      expectedTransitions.slice(-1).updatedAt = new Date();

      expect(stateTransition.getTransitions()).to.deep.equal(
        expectedTransitions,
      );
    });

    it('should create DocumentsBatchTransition with passed documents - Rust', () => {
      const [newDocument] = documents;


      const stateTransition = factory.createStateTransition({
        create: documents,
        // replace: [newDocument],
      }, dataContract);

      console.log(stateTransition.toJSON());

      // const expectedTransitions = getDocumentTransitionsFixture({
      //   create: documentsJs,
      //   replace: [newDocument],
      // });

      // expectedTransitions.slice(-1).updatedAt = new Date();

      // expect(stateTransition.getTransitions()).to.deep.equal(
      //   expectedTransitions,
      // );
    });
  });
});
