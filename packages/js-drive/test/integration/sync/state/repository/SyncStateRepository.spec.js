const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');
const SyncState = require('../../../../../lib/sync/state/SyncState');
const SyncStateRepository = require('../../../../../lib/sync/state/repository/SyncStateRepository');
const getBlockFixtures = require('../../../../../lib/test/fixtures/getBlocksFixture');

describe('SyncStateRepository', function main() {
  this.timeout(90000);

  let mongoDatabase;
  let syncStateCollection;
  let syncStateRepository;
  let syncState;

  startMongoDb().then((mongoDb) => {
    mongoDatabase = mongoDb.getDb();
    syncStateCollection = mongoDatabase.collection('syncState');
  });

  beforeEach(async () => {
    const blocks = getBlockFixtures();

    syncState = new SyncState(blocks, new Date(), new Date());
    syncStateRepository = new SyncStateRepository(mongoDatabase);
  });

  it('should store a state', async () => {
    await syncStateRepository.store(syncState);

    const dataFromMongoDb = await syncStateCollection.findOne(
      SyncStateRepository.mongoDbCondition,
    );

    // eslint-disable-next-line no-underscore-dangle
    delete dataFromMongoDb._id;

    expect(dataFromMongoDb).to.deep.equal(syncState.toJSON());
  });

  it('should fetch an updated state', async () => {
    syncState.setLastInitialSyncAt(new Date('2018-01-01'));
    await syncStateCollection.updateOne(
      SyncStateRepository.mongoDbCondition,
      { $set: syncState.toJSON() },
      { upsert: true },
    );

    const stateFromMongo = await syncStateRepository.fetch();

    expect(stateFromMongo.toJSON()).to.deep.equal(syncState);
  });

  it('should fetch an empty state if it is not present', async () => {
    const stateFromMongo = await syncStateRepository.fetch();

    expect(stateFromMongo.toJSON()).to.deep.equal({
      blocks: [],
      lastSyncAt: null,
      lastInitialSyncAt: null,
    });
  });
});
