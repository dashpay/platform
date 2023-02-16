const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const DashPlatformProtocolJs = require('@dashevo/dpp/lib/DashPlatformProtocol');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const MissingOptionErrorJs = require('@dashevo/dpp/lib/errors/MissingOptionError');

const { default: loadWasmDpp } = require('../../../dist');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

let Document;
let DataContract;
let Identifier;
let ValidationResult;
let DocumentsBatchTransition;
let DashPlatformProtocol;
let DataContractNotPresentError;

describe('DocumentFacade', () => {
  let dppJs;
  let dpp;
  let documentJs;
  let document;
  let documentsJs;
  let documents;
  let dataContractJs;
  let dataContract;
  let ownerIdJs;
  let ownerId;
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let blsAdapter;

  beforeEach(async function beforeEach() {
    ({
      Document,
      DataContract,
      Identifier,
      ValidationResult,
      DocumentsBatchTransition,
      DashPlatformProtocol,
      DataContractNotPresentError,
    } = await loadWasmDpp());

    ownerIdJs = generateRandomIdentifier();
    ownerId = new Identifier(ownerIdJs.toBuffer());
    dataContractJs = getDataContractFixture(ownerIdJs);
    dataContract = new DataContract(dataContractJs.toObject());

    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMockJs.fetchDataContract.resolves(dataContractJs);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    dppJs = new DashPlatformProtocolJs({
      stateRepository: stateRepositoryMockJs,
    });
    await dppJs.initialize();

    blsAdapter = await getBlsAdapterMock();
    dpp = new DashPlatformProtocol(blsAdapter, stateRepositoryMock);

    documentsJs = getDocumentsFixture(dataContractJs);
    documents = documentsJs.map((d) => {
      const currentDocument = new Document(d.toObject(), dataContract.clone());
      currentDocument.setEntropy(d.entropy);
      return currentDocument;
    });
    ([documentJs] = documentsJs);
    ([document] = documents);
  });

  describe('create', () => {
    it('should create Document - Rust', async () => {
      const result = dpp.document.create(
        dataContract,
        ownerId,
        document.getType(),
        document.getData(),
      );

      expect(result).to.be.an.instanceOf(Document);

      expect(result.getType()).to.equal(document.getType());
      expect(result.getData()).to.deep.equal(document.getData());
    });
  });

  describe('createFromObject', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dppJs = new DashPlatformProtocolJs();
      await dppJs.initialize();

      try {
        await dppJs.document.createFromObject(documentJs.toObject());

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionErrorJs);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should throw MissingOption if stateRepository is not set - Rust', async () => {
      // not applicable
    });

    it('should create Document from plain object - Rust', async () => {
      const result = await dpp.document.createFromObject(document.toObject());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toObject()).to.deep.equal(document.toObject());
    });
  });

  describe('createFromBuffer', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dppJs = new DashPlatformProtocolJs();
      await dppJs.initialize();

      try {
        await dppJs.document.createFromBuffer(documentJs.toObject());

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionErrorJs);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should throw MissingOption if stateRepository is not set - Rust', async () => {
      // not applicable
    });

    it('should create Document from serialized - Rust', async () => {
      const result = await dpp.document.createFromBuffer(document.toBuffer());

      expect(result).to.be.an.instanceOf(Document);

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

  describe('validate', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dppJs = new DashPlatformProtocolJs();
      await dppJs.initialize();

      try {
        await dppJs.document.validate(documentJs);

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionErrorJs);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should throw MissingOption if stateRepository is not set - Rust', async () => {
      // not applicable
    });

    it('should validate Document - Rust', async () => {
      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if Data Contract is invalid', async () => {
      stateRepositoryMock.fetchDataContract.resolves(null);

      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataContractNotPresentError);
    });
  });
});
