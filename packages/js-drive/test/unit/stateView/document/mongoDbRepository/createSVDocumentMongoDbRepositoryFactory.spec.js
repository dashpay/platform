const createSVDocumentMongoDbRepositoryFactory = require('../../../../../lib/stateView/document/mongoDbRepository/createSVDocumentMongoDbRepositoryFactory');

describe('createSVDocumentMongoDbRepositoryFactory', () => {
  let mongoClient;
  let mongoDb;
  let createSVDocumentMongoDbRepository;
  let contractId;
  let documentType;
  let SVDocumentMongoDbRepositoryMock;
  let convertWhereToMongoDbQuery;
  let validateQuery;

  beforeEach(function beforeEach() {
    contractId = 'HgKXrLhm7sMjPrRGS1UsETmmQ7nZHbaKN729zw55PUVk';
    documentType = 'niceDocument';

    mongoDb = {};
    mongoClient = {
      db: this.sinon.stub().returns(mongoDb),
    };

    SVDocumentMongoDbRepositoryMock = this.sinon.stub();

    convertWhereToMongoDbQuery = this.sinon.stub();
    validateQuery = this.sinon.stub();

    createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
      mongoClient,
      SVDocumentMongoDbRepositoryMock,
      convertWhereToMongoDbQuery,
      validateQuery,
    );
  });

  it('should create a MongoDb database with a prefix + contractId', async () => {
    const dbName = `${process.env.STATEVIEW_MONGODB_DB_PREFIX}dpa_${contractId}`;

    const result = createSVDocumentMongoDbRepository(contractId, documentType);

    expect(result).to.be.an.instanceof(SVDocumentMongoDbRepositoryMock);

    expect(mongoClient.db).to.have.been.calledOnceWith(dbName);

    expect(SVDocumentMongoDbRepositoryMock).to.have.been.calledOnceWith(
      mongoDb,
      convertWhereToMongoDbQuery,
      validateQuery,
      documentType,
    );
  });
});
