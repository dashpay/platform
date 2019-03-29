const createSVDocumentMongoDbRepositoryFactory = require('../../../../lib/stateView/document/createSVDocumentMongoDbRepositoryFactory');

describe('createSVDocumentMongoDbRepositoryFactory', () => {
  let mongoClient;
  let mongoDb;
  let createSVDocumentMongoDbRepository;
  let contractId;
  let documentType;
  let SVDocumentMongoDbRepositoryMock;
  let sanitizerMock;

  beforeEach(function beforeEach() {
    contractId = 'HgKXrLhm7sMjPrRGS1UsETmmQ7nZHbaKN729zw55PUVk';
    documentType = 'niceDocument';

    mongoDb = {};
    mongoClient = {
      db: this.sinon.stub().returns(mongoDb),
    };

    sanitizerMock = {};

    SVDocumentMongoDbRepositoryMock = this.sinon.stub();

    createSVDocumentMongoDbRepository = createSVDocumentMongoDbRepositoryFactory(
      mongoClient,
      SVDocumentMongoDbRepositoryMock,
      sanitizerMock,
    );
  });

  it('should create a MongoDb database with a prefix + contractId', async () => {
    const dbName = `${process.env.MONGODB_DB_PREFIX}dpa_${contractId}`;

    const result = createSVDocumentMongoDbRepository(contractId, documentType);

    expect(result).to.be.an.instanceof(SVDocumentMongoDbRepositoryMock);

    expect(mongoClient.db).to.have.been.calledOnceWith(dbName);

    expect(SVDocumentMongoDbRepositoryMock).to.have.been.calledOnceWith(
      mongoDb,
      sanitizerMock,
      documentType,
    );
  });
});
