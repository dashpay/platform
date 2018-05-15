const startMongoDbInstance = require('../../../../../lib/test/services/mongoDb/startMongoDbInstance');

describe('startMongoDbInstance', function main() {
  this.timeout(90000);

  describe('One instance', () => {
    let instance;

    before(async () => {
      instance = await startMongoDbInstance();
    });
    after(async () => instance.remove());

    it('should has MongoDb container running', async () => {
      const { State } = await instance.container.details();
      expect(State.Status).to.equal('running');
    });
  });

  describe('Three instance', () => {
    let instances;

    before(async () => {
      instances = await startMongoDbInstance.many(3);
    });
    after(async () => {
      const promises = instances.map(instance => instance.remove());
      await Promise.all(promises);
    });

    it('should have MongoDb containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].container.details();
        expect(State.Status).to.equal('running');
      }
    });
  });
});
