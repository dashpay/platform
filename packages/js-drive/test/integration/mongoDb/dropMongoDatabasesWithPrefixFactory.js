const { mocha: { startMongoDb } } = require('@dashevo/js-evo-services-ctl');

const dropMongoDatabasesWithPrefixFactory = require('../../../lib/mongoDb/dropMongoDatabasesWithPrefixFactory');

const byDbPrefix = prefix => db => db.name.includes(prefix);

describe('dropMongoDatabasesWithPrefixFactory', () => {
  let mongoClient;

  startMongoDb().then((instance) => {
    mongoClient = instance.getClient();
  });

  it('should drop all Drive Mongo databases', async () => {
    await mongoClient.db('drive_db').collection('something').insertOne({ name: 'DashPay' });

    const { databases: dbs } = await mongoClient.db('test').admin().listDatabases();
    const filterDb = dbs.filter(byDbPrefix(process.env.MONGODB_DB_PREFIX));
    expect(filterDb.length).to.equal(1);

    const dropMongoDatabasesWithPrefix = dropMongoDatabasesWithPrefixFactory(mongoClient);
    await dropMongoDatabasesWithPrefix(process.env.MONGODB_DB_PREFIX);

    const { databases: dbsAfter } = await mongoClient.db('test').admin().listDatabases();
    const filterDbAfter = dbsAfter.filter(byDbPrefix(process.env.MONGODB_DB_PREFIX));
    expect(filterDbAfter.length).to.equal(0);
  });
});
