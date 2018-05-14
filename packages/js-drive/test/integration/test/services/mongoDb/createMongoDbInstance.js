const Docker = require('dockerode');
const createMongoDbInstance = require('../../../../../lib/test/services/mongoDb/createMongoDbInstance');

describe('createMongoDbInstance', function main() {
  this.timeout(40000);

  describe('usage', () => {
    let instance;

    before(async () => {
      instance = await createMongoDbInstance();
    });
    after(async () => instance.remove());

    it('should start an instance with a bridge dash_test_network', async () => {
      await instance.start();
      const network = new Docker().getNetwork('dash_test_network');
      const { Driver } = await network.inspect();
      const { NetworkSettings: { Networks } } = await instance.container.details();
      const networks = Object.keys(Networks);
      expect(Driver).to.equal('bridge');
      expect(networks.length).to.equal(1);
      expect(networks[0]).to.equal('dash_test_network');
    });

    it('should start an instance with the default options', async () => {
      await instance.start();
      const { Args } = await instance.container.details();
      expect(Args).to.deep.equal([
        'mongod',
      ]);
    });

    it('should get Mongo db client', async () => {
      await instance.start();

      const client = await instance.getMongoClient();
      const db = client.collection('syncState');
      const count = await db.count({});

      expect(count).to.equal(0);
    });

    it('should clean Mongo database', async () => {
      await instance.start();

      const client = await instance.getMongoClient();
      const db = client.collection('syncState');
      await db.insertOne({ blocks: [], lastSynced: new Date() });

      const countBefore = await db.count({});
      expect(countBefore).to.equal(1);

      await instance.clean();

      const countAfter = await db.count({});
      expect(countAfter).to.equal(0);
    });
  });

  describe('Mongo client', () => {
    let instance;

    before(async () => {
      instance = await createMongoDbInstance();
    });
    after(async () => instance.remove());

    it('should not fail if mongod is not running yet (MongoNetworkError)', async () => {
      await instance.start();

      const client = await instance.getMongoClient();
      const db = client.collection('syncState');
      const count = await db.count({});

      expect(count).to.equal(0);
    });
  });
});
