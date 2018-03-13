const dotenv = require('dotenv');
const { expect } = require('chai');

const SyncStateRepository = require('../../lib/syncState/SyncStateRepository');
const getBlockFixtures = require('../../lib/test/fixtures/getBlockFixtures');
const connectToMongoDb = require('../../lib/test/connectToMongoDb');

// Will be called in bootstrap
dotenv.config();

// Will be set once in bootstrap
connectToMongoDb.setUrl(process.env.STORAGE_MONGODB_URL)
  .setDbName(process.env.STORAGE_MONGODB_DB);

describe('SyncStateRepository', () => {
  let mongoDb;
  let mongoCollection;
  let stateBlocks;
  let blocks;
  let syncStateRepository;
  let stateMock;

  connectToMongoDb().then((db) => {
    mongoDb = db;
  });

  beforeEach(() => {
    mongoCollection = mongoDb.collection('syncState');

    blocks = getBlockFixtures();

    stateMock = {
      setBlocks(b) {
        stateBlocks = b;
      },
      getBlocks() {
        return stateBlocks;
      },
    };

    syncStateRepository = new SyncStateRepository(mongoDb);
  });

  it('should store state', async () => {
    stateMock.setBlocks(blocks);

    await syncStateRepository.store(stateMock);

    const { blocks: blocksFromMongoDb } =
      await mongoCollection.findOne(SyncStateRepository.mongoDbCondition);

    expect(blocksFromMongoDb).to.be.deep.equals(blocks);
  });

  it('should populate state', async () => {
    await mongoCollection.updateOne(
      SyncStateRepository.mongoDbCondition,
      { $set: { blocks } },
      { upsert: true },
    );

    const stateFromMongo = await syncStateRepository.populate(stateMock);

    expect(stateFromMongo).to.be.equals(stateMock);
    expect(stateFromMongo.getBlocks()).to.be.deep.equals(blocks);
  });
});
