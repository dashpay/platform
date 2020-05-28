const { MongoClient } = require('mongodb');

const { startMongoDb } = require('@dashevo/dp-services-ctl');

const connectToMongoDBFactory = require('../../../lib/mongoDb/connectToMongoDBFactory');

describe('connectToMongoDBFactory', function describeContainer() {
  this.timeout(20000);

  let mongoDB;
  let connectToMongoDB;
  let connectionUrl;

  before(async () => {
    mongoDB = await startMongoDb();
  });

  after(async () => {
    await mongoDB.remove();
  });

  beforeEach(() => {
    connectionUrl = `mongodb://127.0.0.1:${mongoDB.options.getMongoPort()}`;
    connectToMongoDB = connectToMongoDBFactory(connectionUrl);
  });

  it('should return mongo db client', async () => {
    const mongoClient = await connectToMongoDB();
    expect(mongoClient).to.be.an.instanceOf(MongoClient);
    expect(mongoClient.isConnected()).to.be.true();
  });
});
