const bs58 = require('bs58');

const createSVObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/object/createSVObjectMongoDbRepositoryFactory');

describe('createSVObjectMongoDbRepositoryFactory', () => {
  let mongoClient;
  let mongoDb;
  let createSVObjectMongoDbRepository;
  let contractId;
  let objectType;
  let SVObjectMongoDbRepositoryMock;
  let sanitizerMock;

  beforeEach(function beforeEach() {
    contractId = 'b8ae412cdeeb4bb39ec496dec34495ecccaf74f9fa9eaa712c77a03eb1994e75';
    objectType = 'niceObject';

    mongoDb = {};
    mongoClient = {
      db: this.sinon.stub().returns(mongoDb),
    };

    sanitizerMock = {};

    SVObjectMongoDbRepositoryMock = this.sinon.stub();

    createSVObjectMongoDbRepository = createSVObjectMongoDbRepositoryFactory(
      mongoClient,
      SVObjectMongoDbRepositoryMock,
      sanitizerMock,
    );
  });

  it('should create a MongoDb database with a prefix + contractId', async () => {
    const contractIdEncoded = bs58.encode(Buffer.from(contractId, 'hex'));
    const dbName = `${process.env.MONGODB_DB_PREFIX}dpa_${contractIdEncoded}`;

    const result = createSVObjectMongoDbRepository(contractId, objectType);

    expect(result).to.be.an.instanceof(SVObjectMongoDbRepositoryMock);

    expect(mongoClient.db).to.have.been.calledOnceWith(dbName);

    expect(SVObjectMongoDbRepositoryMock).to.have.been.calledOnceWith(
      mongoDb,
      sanitizerMock,
      objectType,
    );
  });
});
