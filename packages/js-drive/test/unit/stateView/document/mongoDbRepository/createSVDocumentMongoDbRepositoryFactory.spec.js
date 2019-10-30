const bs58 = require('bs58');

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
    contractId = Buffer.alloc(32, 'somePool').toString('hex');
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
    const base58ContractId = bs58.encode(
      Buffer.from(contractId, 'hex'),
    );

    const dbName = `${process.env.STATEVIEW_MONGODB_DB_PREFIX}dpa_${base58ContractId}`;

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
