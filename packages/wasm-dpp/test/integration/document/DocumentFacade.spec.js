const crypto = require('crypto');
const generateRandomIdentifier = require('../../../lib/test/fixtures/js/generateRandomIdentifier');
const getDataContractFixture = require('../../../lib/test/fixtures/js/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/js/getDocumentsFixture');
const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');
const getDocumentTransitionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const { default: loadWasmDpp } = require('../../../dist');

let ExtendedDocument;
let DataContract;
let Identifier;
let ValidationResult;
let DocumentsBatchTransition;
let DashPlatformProtocol;
let DataContractNotPresentError;

describe('DocumentFacade', () => {
  let dpp;
  let document;
  let documents;
  let documentsJs;
  let dataContract;
  let ownerId;
  let stateRepositoryMock;

  beforeEach(async function beforeEach() {
    ({
      ExtendedDocument,
      DataContract,
      Identifier,
      ValidationResult,
      DocumentsBatchTransition,
      DashPlatformProtocol,
      DataContractNotPresentError,
    } = await loadWasmDpp());

    const ownerIdJs = generateRandomIdentifier();
    ownerId = new Identifier(ownerIdJs.toBuffer());
    const dataContractJs = getDataContractFixture(ownerIdJs);
    dataContract = new DataContract(dataContractJs.toObject());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
      1,
    );

    documentsJs = getDocumentsFixture(dataContractJs);
    documents = documentsJs.map((d) => {
      const currentDocument = new ExtendedDocument(d.toObject(), dataContract.clone());
      currentDocument.setEntropy(d.entropy);
      return currentDocument;
    });
    ([document] = documents);
  });

  describe('create', () => {
    it('should create Document - Rust', async () => {
      const documentType = document.getType();
      const documentData = document.getData();

      const result = dpp.document.create(
        dataContract,
        ownerId,
        documentType,
        documentData,
      );

      expect(result).to.be.an.instanceOf(ExtendedDocument);

      expect(result.getType()).to.equal(document.getType());
      expect(result.getData()).to.deep.equal(document.getData());
    });
  });

  describe.skip('createFromObject', () => {
    it('should throw MissingOption if stateRepository is not set - Rust', async () => {
      // not applicable
    });

    it('should create Document from plain object - Rust', async () => {
      const a = document.toObject();
      const result = await dpp.document.createFromObject(a);

      expect(result).to.be.an.instanceOf(ExtendedDocument);

      expect(result.toObject()).to.deep.equal(document.toObject());
    });
  });

  describe.skip('createFromBuffer', () => {
    it('should throw MissingOption if stateRepository is not set - Rust', async () => {
      // not applicable
    });

    it('should create Document from serialized - Rust', async () => {
      const result = await dpp.document.createFromBuffer(document.toBuffer());

      expect(result).to.be.an.instanceOf(ExtendedDocument);

      expect(result.toObject()).to.deep.equal(document.toObject());
    });
  });

  describe('createStateTransition', () => {
    it('should create DocumentsBatchTransition with passed documents - Rust', () => {
      const result = dpp.document.createStateTransition({
        create: documents,
      });

      expect(result).to.be.instanceOf(DocumentsBatchTransition);
      expect(result.getTransitions().map((t) => t.toObject()))
        .has.deep.members(getDocumentTransitionsFixture({
          create: documentsJs,
        }).map((t) => t.toObject()));
    });
  });

  describe.skip('validate', () => {
    it('should throw MissingOption if stateRepository is not set - Rust', async () => {
      // not applicable
    });

    it('should validate Document - Rust', async () => {
      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if Data Contract is invalid - Rust', async () => {
      stateRepositoryMock.fetchDataContract.resolves(null);

      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataContractNotPresentError);
    });
  });
});
