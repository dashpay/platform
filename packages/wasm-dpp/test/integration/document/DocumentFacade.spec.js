const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const DashPlatformProtocolJs = require('@dashevo/dpp/lib/DashPlatformProtocol');

const DocumentJs = require('@dashevo/dpp/lib/document/Document');
const DocumentsBatchTransitionJs = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const ValidationResultJs = require('@dashevo/dpp/lib/validation/ValidationResult');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const DataContractNotPresentErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/DataContractNotPresentError');
const MissingOptionErrorJs = require('@dashevo/dpp/lib/errors/MissingOptionError');

const { default: loadWasmDpp } = require('../../../dist');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

let Document;
let DataContract;
let ValidationResult;
let DocumentsBatchTransition;
let DashPlatformProtocol;
let DocumentFactory;
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
      DocumentFactory,
      DataContractNotPresentError,
    } = await loadWasmDpp());

    ownerIdJs = generateRandomIdentifier();
    ownerId = new Identifier(ownerIdJs.toBuffer());
    dataContractJs = getDataContractFixture(ownerIdJs);
    dataContract = new DataContract(dataContractJs.toObject());

    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMockJs.fetchDataContract.resolves(dataContractJs);
    stateRepositoryMock.fetchDataContract.returns(dataContract);


    dppJs = new DashPlatformProtocolJs({
      stateRepository: stateRepositoryMockJs,
    });
    await dppJs.initialize();

    blsAdapter = await getBlsAdapterMock();
    dpp = new DashPlatformProtocol(blsAdapter, stateRepositoryMock);

    documentsJs = getDocumentsFixture(dataContractJs);
    documents = documentsJs.map((d) => {
      const document = new Document(d.toObject(), dataContract.clone())
      document.setEntropy(d.entropy);
      return document;
    });
    ([documentJs] = documentsJs);
    ([document] = documents);
  });

  describe('create', () => {
    it('should create Document', () => {
      const result = dppJs.document.create(
        dataContractJs,
        ownerIdJs,
        documentJs.getType(),
        documentJs.getData(),
      );

      expect(result).to.be.an.instanceOf(DocumentJs);

      expect(result.getType()).to.equal(documentJs.getType());
      expect(result.getData()).to.deep.equal(documentJs.getData());
    });

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

    it('should create Document from plain object', async () => {
      const result = await dppJs.document.createFromObject(documentJs.toObject());

      expect(result).to.be.an.instanceOf(DocumentJs);

      expect(result.toObject()).to.deep.equal(documentJs.toObject());
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

    it('should create Document from serialized', async () => {
      const result = await dppJs.document.createFromBuffer(documentJs.toBuffer());

      expect(result).to.be.an.instanceOf(DocumentJs);

      expect(result.toObject()).to.deep.equal(documentJs.toObject());
    });

    it('should create Document from serialized - Rust', async () => {
      const result = await dpp.document.createFromBuffer(document.toBuffer());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toObject()).to.deep.equal(document.toObject());
    });
  });

  describe('createStateTransition', () => {
    it('should create DocumentsBatchTransition with passed documents', () => {
      const result = dppJs.document.createStateTransition({
        create: documentsJs,
      });

      expect(result).to.be.instanceOf(DocumentsBatchTransitionJs);
      expect(result.getTransitions()).to.deep.equal(getDocumentTransitionsFixture({
        create: documentsJs,
      }));
    });

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


    it('should validate Document', async () => {
      const result = await dppJs.document.validate(documentJs);

      expect(result).to.be.an.instanceOf(ValidationResultJs);
      expect(result.isValid()).to.be.true();
    });

    it('should validate Document - Rust', async () => {
      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if Data Contract is invalid', async () => {
      stateRepositoryMockJs.fetchDataContract.returns(null);

      const result = await dppJs.document.validate(documentJs);

      expect(result).to.be.an.instanceOf(ValidationResultJs);
      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataContractNotPresentErrorJs);
    });

    it('should return invalid result if Data Contract is invalid - Rust', async () => {
      stateRepositoryMock.fetchDataContract.returns(null);

      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(DataContractNotPresentError);
    })
  });
});
