const Docker = require('dockerode');
const createMongoDbInstance = require('../../../../../lib/test/services/mongoDb/createMongoDbInstance');

describe('createMongoDbInstance', function main() {
  this.timeout(40000);

  describe('usage', () => {
    let instance;

    before(async () => {
      instance = await createMongoDbInstance();
    });
    after(async () => instance.clean());

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
  });
});
