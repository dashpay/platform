const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

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
  let dataContractRepositoryMock;
  let dataContract;
  let containerMock;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    contractId = dataContract.getId();
    documentType = 'niceDocument';

    mongoDb = {
      collection: this.sinon.stub(),
    };

    convertWhereToMongoDbQuery = this.sinon.stub();
    validateQuery = this.sinon.stub();
    getDocumentsDatabaseMock = this.sinon.stub().resolves(mongoDb);
    dataContractRepositoryMock = {
      fetch: this.sinon.stub().resolves(dataContract),
    };

    containerMock = {
      resolve: this.sinon.stub().returns({
        getTransaction: this.sinon.stub(),
      }),
    };

    createDocumentMongoDbRepository = createDocumentMongoDbRepositoryFactory(
      convertWhereToMongoDbQuery,
      validateQuery,
      getDocumentsDatabaseMock,
      dataContractRepositoryMock,
      containerMock,
    );
  });

  it('should create a MongoDb database with a prefix + contractId', async () => {
    const result = await createDocumentMongoDbRepository(contractId, documentType);

    expect(result).to.be.an.instanceof(DocumentMongoDbRepository);
    expect(result.mongoDatabase).to.deep.equal(mongoDb);
    expect(result.convertWhereToMongoDbQuery).to.deep.equal(convertWhereToMongoDbQuery);
    expect(result.validateQuery).to.deep.equal(validateQuery);
    expect(result.documentType).to.equal(documentType);
    expect(result.dataContract).to.deep.equal(dataContract);
  });
});
