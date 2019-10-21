const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const Document = require('../../../lib/document/Document');
const DocumentsStateTransition = require('../../../lib/document/stateTransition/DocumentsStateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const MissingDocumentContractIdError = require('../../../lib/errors/MissingDocumentContractIdError');
const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('DocumentFacade', () => {
  let dpp;
  let document;
  let documents;
  let dataContract;
  let userId;
  let dataProviderMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    userId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    dataProviderMock.fetchDataContract.resolves(dataContract);

    dpp = new DashPlatformProtocol({
      dataProvider: dataProviderMock,
    });

    documents = getDocumentsFixture();
    ([document] = documents);
  });

  describe('create', () => {
    it('should create Document', () => {
      const result = dpp.document.create(
        dataContract,
        userId,
        document.getType(),
        document.getData(),
      );

      expect(result).to.be.an.instanceOf(Document);

      expect(result.getType()).to.equal(document.getType());
      expect(result.getData()).to.deep.equal(document.getData());
    });
  });

  describe('createFromObject', () => {
    it('should throw MissingOption if dataProvider is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.document.createFromObject(document.toJSON());

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('dataProvider');
      }
    });

    it('should create Document from plain object', async () => {
      const result = await dpp.document.createFromObject(document.toJSON());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toJSON()).to.deep.equal(document.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should throw MissingOption if dataProvider is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.document.createFromSerialized(document.toJSON());

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('dataProvider');
      }
    });

    it('should create Document from string', async () => {
      const result = await dpp.document.createFromSerialized(document.serialize());

      expect(result).to.be.an.instanceOf(Document);

      expect(result.toJSON()).to.deep.equal(document.toJSON());
    });
  });

  describe('createStatTransition', () => {
    it('should create DocumentsStateTransition with passed documents', () => {
      const result = dpp.document.createStateTransition(documents);

      expect(result).to.be.instanceOf(DocumentsStateTransition);
      expect(result.getDocuments()).to.equal(documents);
    });
  });

  describe('validate', () => {
    it('should throw MissingOption if dataProvider is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.document.validate(document);

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('dataProvider');
      }
    });

    it('should validate Document', async () => {
      const result = await dpp.document.validate(document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should return invalid result if Data Contract is invalid', async () => {
      dataProviderMock.fetchDataContract.returns(null);

      const result = await dpp.document.validate(dataContract, document);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      const [error] = result.getErrors();

      expect(error).to.be.an.instanceOf(MissingDocumentContractIdError);
    });
  });
});
