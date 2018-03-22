const dotenv = require('dotenv');
const { expect } = require('chai');

const SyncState = require('../../../../lib/sync/state/SyncState');
const SyncStateRepository = require('../../../../lib/sync/state/repository/SyncStateRepository');
const getBlockFixtures = require('../../../../lib/test/fixtures/getBlockFixtures');
const connectToMongoDb = require('../../../../lib/test/connectToMongoDb');

// Will be called in bootstrap
dotenv.config();

// Will be set once in bootstrap
connectToMongoDb.setUrl(process.env.STORAGE_MONGODB_URL)
  .setDbName(process.env.STORAGE_MONGODB_DB);

describe('SyncStateRepository', () => {
  let mongoDb;
  let mongoCollection;
  let syncStateRepository;
  let syncState;

  connectToMongoDb().then((db) => {
    mongoDb = db;
  });

  beforeEach(() => {
    mongoCollection = mongoDb.collection('syncState');

    const blocks = getBlockFixtures();

    syncState = new SyncState(blocks, new Date());

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
