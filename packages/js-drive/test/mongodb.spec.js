// TODO: Remove it later
const dotenv = require('dotenv');
const { expect } = require('chai');
const connectToMongoDb = require('../lib/test/connectToMongoDb');

// Will be called in bootstrap
dotenv.config();

// Will be set once in bootstrap
connectToMongoDb.setUrl(process.env.STORAGE_MONGODB_URL)
  .setDbName(process.env.STORAGE_MONGODB_DB);

describe('MongoDB', () => {
  let mongoDb;
  connectToMongoDb().then((db) => {
    mongoDb = db;
  });

  it('should work :)', async () => {
    const expectedData = { test: 1 };

    const collection = mongoDb.collection('testCollection');

    await collection.insertOne(expectedData);
    const [actualData] = await collection.find({}).toArray();

    expect(actualData).to.be.deep.equal(expectedData);
  });
});
