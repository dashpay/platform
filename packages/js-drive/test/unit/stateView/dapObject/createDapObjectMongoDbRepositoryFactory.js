const createDapObjectMongoDbRepositoryFactory = require('../../../../lib/stateView/dapObject/createDapObjectMongoDbRepositoryFactory');

describe('createDapObjectMongoDbRepositoryFactory', () => {
  class DapObjectMongoDbRepository {}
  let mongoClient;
  let createDapObjectMongoDbRepository;

  beforeEach(function beforeEach() {
    mongoClient = {
      db: this.sinon.spy(),
      collection: this.sinon.spy(),
    };
    createDapObjectMongoDbRepository = createDapObjectMongoDbRepositoryFactory(
      mongoClient,
      DapObjectMongoDbRepository,
    );
  });

  it('should create Mongo database with prefix + dapId', async () => {
    const dapId = '123456';
    createDapObjectMongoDbRepository(dapId);

    expect(mongoClient.db).to.be.calledWith(`${process.env.MONGODB_DB_PREFIX}dap_77em`);
  });

  it('should create DapObjectMongoDbRepository instance', async () => {
    const dapId = '123456';
    const repository = createDapObjectMongoDbRepository(dapId);

    expect(repository).to.be.an.instanceof(DapObjectMongoDbRepository);
  });
});
