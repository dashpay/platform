const startMongoDbInstance = require('../../../../../lib/test/services/mocha/startMongoDbInstance');

describe('startMongoDbInstance', () => {
  describe('One instance', () => {
    let instance;
    startMongoDbInstance().then((_instance) => {
      instance = _instance;
    });

    it('should start one instance and insert with MongoClient', async () => {
      const client = await instance.getMongoClient();
      const db = client.collection('syncState');
      await db.insertOne({
        blocks: [],
        lastSynced: new Date(),
      });

      const countBefore = await db.count({});
      expect(countBefore).to.equal(1);
    });

    it('should drop MongoDb after last test', async () => {
      const client = await instance.getMongoClient();
      const db = client.collection('syncState');

      const countBefore = await db.count({});
      expect(countBefore).to.equal(0);
    });
  });

  describe('Three instance', () => {
    let instances;
    startMongoDbInstance.many(3).then((_instances) => {
      instances = _instances;
    });

    it('should have MongoDb containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].container.details();
        expect(State.Status).to.equal('running');
      }
    });
  });
});
