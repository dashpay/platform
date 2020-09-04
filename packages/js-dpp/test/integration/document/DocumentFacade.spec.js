const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const Document = require('../../../lib/document/Document');
const DocumentsBatchTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const MissingDataContractIdError = require('../../../lib/errors/MissingDataContractIdError');
const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('DocumentFacade', () => {
  let dpp;
  let document;
  let documents;
  let dataContract;
  let ownerId;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    ownerId = '5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq';

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
    });

    documents = getDocumentsFixture(dataContract);
    ([document] = documents);
  });

  describe('create', () => {
    it('should create Document', () => {
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
      dpp = new DashPlatformProtocol();

      try {
        await dpp.document.createFromObject(document.toJSON());

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should create Document from plain object', async () => {
      const result = await dpp.document.createFromObject(document.toJSON());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toJSON()).to.deep.equal(document.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.document.createFromSerialized(document.toJSON());

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should create Document from string', async () => {
      const result = await dpp.document.createFromSerialized(document.serialize());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toJSON()).to.deep.equal(document.toJSON());
    });
  });

  describe('createStateTransition', () => {
    it('should create DocumentsBatchTransition with passed documents', () => {
      const result = dpp.document.createStateTransition({
        create: documents,
      });

      expect(result).to.be.instanceOf(DocumentsBatchTransition);
      expect(result.getTransitions()).to.deep.equal(getDocumentTransitionsFixture({
        create: documents,
      }));
    });
  });

  describe('validate', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.document.validate(document);

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate Document', async () => {
      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if Data Contract is invalid', async () => {
      stateRepositoryMock.fetchDataContract.returns(null);

      const result = await dpp.document.validate(dataContract, document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(MissingDataContractIdError);
    });
  });
});
