const startMongoDbInstance = require('../../../lib/test/services/mocha/startMongoDbInstance');
const dropMongoDatabasesWithPrefixFactory = require('../../../lib/mongoDb/dropMongoDatabasesWithPrefixFactory');

const byDbPrefix = prefix => db => db.name.includes(prefix);

describe('dropMongoDatabasesWithPrefixFactory', () => {
  let mongoClient;
  startMongoDbInstance().then((instance) => {
    ({ mongoClient } = instance);
  });

  it('should drop all Drive Mongo databases', async () => {
    await mongoClient.db('drive_db').collection('dapObjects').insertOne({ name: 'DashPay' });

    const { databases: dbs } = await mongoClient.db().admin().listDatabases();
    const filterDb = dbs.filter(byDbPrefix(process.env.STORAGE_MONGODB_DB_PREFIX));
    expect(filterDb.length).to.equal(1);

    const dropMongoDatabasesWithPrefix = dropMongoDatabasesWithPrefixFactory(mongoClient);
    await dropMongoDatabasesWithPrefix(process.env.STORAGE_MONGODB_DB_PREFIX);

    const { databases: dbsAfter } = await mongoClient.db().admin().listDatabases();
    const filterDbAfter = dbsAfter.filter(byDbPrefix(process.env.STORAGE_MONGODB_DB_PREFIX));
    expect(filterDbAfter.length).to.equal(0);
  });
});
