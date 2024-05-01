const crypto = require('crypto');
const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifierAsync');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const {
  ExtendedDocument,
  DataContract,
  ValidationResult,
  DocumentsBatchTransition,
  DashPlatformProtocol,
  DataContractNotPresentError,
} = require('../../..');

describe('DocumentFacade', () => {
  let dpp;
  let document;
  let documents;
  let dataContract;
  let ownerId;
  let stateRepositoryMock;

  beforeEach(async function beforeEach() {
    ownerId = await generateRandomIdentifier();
    const dataContractFixture = await getDataContractFixture(ownerId.toBuffer());
    const dataContractObject = dataContractFixture.toObject();

    dataContract = new DataContract({
      $format_version: '0',
      id: dataContractObject.id,
      version: 1,
      ownerId: dataContractObject.ownerId,
      documentSchemas: dataContractObject.documentSchemas,
    });

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
      1,
    );

    documents = await getDocumentsFixture(dataContract);
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
      const identityId = documents[0].getOwnerId();
      const contractId = documents[0].getDataContractId();

      const result = dpp.document.createStateTransition({
        create: documents,
      }, {
        [identityId.toString()]: {
          [contractId.toString()]: 1,
        },
      });

      expect(result).to.be.instanceOf(DocumentsBatchTransition);
      // expect(result.getTransitions().map((t) => t.toObject()))
      //   .has.deep.members(getDocumentTransitionsFixture({
      //     create: documentsJs,
      //   }).map((t) => t.toObject()));
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
