const SyncState = require('../../../../../lib/sync/state/SyncState');
const SyncStateRepository = require('../../../../../lib/sync/state/repository/SyncStateRepository');
const getBlockFixtures = require('../../../../../lib/test/fixtures/getBlockFixtures');
const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

describe('SyncStateRepository', function main() {
  this.timeout(90000);

  let mongoDb;
  let mongoCollection;
  let syncStateRepository;
  let syncState;
  let instance;

  startMongoDb().then((_instance) => {
    instance = _instance;
  });

  beforeEach(async () => {
    mongoDb = await instance.getDb();
    mongoCollection = mongoDb.collection('syncState');
    const blocks = getBlockFixtures();
    syncState = new SyncState(blocks, new Date(), new Date());
    syncStateRepository = new SyncStateRepository(mongoDb);
  });

  it('should store state', async () => {
    await syncStateRepository.store(syncState);

    const dataFromMongoDb =
      await mongoCollection.findOne(SyncStateRepository.mongoDbCondition);

    // eslint-disable-next-line no-underscore-dangle
    delete dataFromMongoDb._id;

    expect(dataFromMongoDb).to.be.deep.equals(syncState.toJSON());
  });

  it('should fetch updated state', async () => {
    syncState.setLastInitialSyncAt(new Date('2018-01-01'));
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
      lastInitialSyncAt: null,
    });
  });
});

