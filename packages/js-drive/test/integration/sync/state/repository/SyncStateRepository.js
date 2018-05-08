const SyncState = require('../../../../../lib/sync/state/SyncState');
const SyncStateRepository = require('../../../../../lib/sync/state/repository/SyncStateRepository');
const getBlockFixtures = require('../../../../../lib/test/fixtures/getBlockFixtures');
const startMongoDbInstance = require('../../../../../lib/test/services/mongoDb/startMongoDbInstance');

describe('SyncStateRepository', function main() {
  this.timeout(90000);

  let mongoDb;
  let mongoCollection;
  let syncStateRepository;
  let syncState;
  let instance;

  before(async () => {
    instance = await startMongoDbInstance();
    mongoDb = await instance.getMongoClient();
  });
  beforeEach(async () => {
    await mongoDb.dropDatabase();
    mongoCollection = mongoDb.collection('syncState');

    const blocks = getBlockFixtures();
    syncState = new SyncState(blocks, new Date());

    syncStateRepository = new SyncStateRepository(mongoDb);
  });
  after(async () => instance.clean());


  it('should store state', async () => {
    await syncStateRepository.store(syncState);

    const dataFromMongoDb =
      await mongoCollection.findOne(SyncStateRepository.mongoDbCondition);

    // eslint-disable-next-line no-underscore-dangle
    delete dataFromMongoDb._id;

    expect(dataFromMongoDb).to.be.deep.equals(syncState.toJSON());
  });

  it('should fetch state', async () => {
    await mongoCollection.updateOne(
      SyncStateRepository.mongoDbCondition,
      { $set: syncState.toJSON() },
      { upsert: true },
    );

    const stateFromMongo = await syncStateRepository.fetch();

    expect(stateFromMongo.toJSON()).to.be.deep.equals(syncState);
  });

  it('should fetch empty state if not present', async () => {
    const stateFromMongo = await syncStateRepository.fetch();

    expect(stateFromMongo.toJSON()).to.be.deep.equals({
      blocks: [],
      lastSyncAt: null,
    });
  });
});

