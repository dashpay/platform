const generateRandomId = require('@dashevo/dpp/lib/test/utils/generateRandomId');

const createDocumentMongoDbRepositoryFactory = require('../../../../lib/document/mongoDbRepository/createDocumentMongoDbRepositoryFactory');
const DocumentMongoDbRepository = require('../../../../lib/document/mongoDbRepository/DocumentMongoDbRepository');

describe('createDocumentMongoDbRepositoryFactory', () => {
  let mongoDb;
  let createDocumentMongoDbRepository;
  let contractId;
  let documentType;
  let convertWhereToMongoDbQuery;
  let validateQuery;
  let getDocumentsDatabaseMock;

  beforeEach(function beforeEach() {
    contractId = generateRandomId();
    documentType = 'niceDocument';

    mongoDb = {
      collection: this.sinon.stub(),
    };

    convertWhereToMongoDbQuery = this.sinon.stub();
    validateQuery = this.sinon.stub();
    getDocumentsDatabaseMock = this.sinon.stub().resolves(mongoDb);

    createDocumentMongoDbRepository = createDocumentMongoDbRepositoryFactory(
      convertWhereToMongoDbQuery,
      validateQuery,
      getDocumentsDatabaseMock,
    );
  });

  it('should create a MongoDb database with a prefix + contractId', async () => {
    const result = await createDocumentMongoDbRepository(contractId, documentType);

    expect(result).to.be.an.instanceof(DocumentMongoDbRepository);
    expect(result.mongoDatabase).to.deep.equal(mongoDb);
    expect(result.convertWhereToMongoDbQuery).to.deep.equal(convertWhereToMongoDbQuery);
    expect(result.validateQuery).to.deep.equal(validateQuery);
    expect(result.documentType).to.equal(documentType);
    expect(result.contractId).to.equal(contractId);
  });
});
