const removeContainers = require('../../../../../lib/test/services/docker/removeContainers');
const startDashCoreInstance = require('../../../../../lib/test/services/dashCore/startDashCoreInstance');

const wait = require('../../../../../lib/test/util/wait');

describe('startDashCoreInstance', function main() {
  this.timeout(40000);

  before(removeContainers);

  describe('One instance', () => {
    let instance;

    before(async () => {
      instance = await startDashCoreInstance();
    });
    after(async () => instance.remove());

    it('should has container running', async () => {
      const { State } = await instance.container.details();
      expect(State.Status).to.equal('running');
    });

    it('should has RPC connected', async () => {
      const { result } = await instance.rpcClient.getInfo();
      expect(result).to.have.property('version');
    });
  });

  describe('Three instances', () => {
    let instances;

    before(async () => {
      instances = await startDashCoreInstance.many(3);
    });
    after(async () => {
      const promises = instances.map(instance => instance.remove());
      await Promise.all(promises);
    });

    it('should have containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].container.details();
        expect(State.Status).to.equal('running');
      }
    });

    it('should propagate blocks between instances', async () => {
      for (let i = 0; i < 3; i++) {
        const { result: blocks } = await instances[i].rpcClient.getBlockCount();
        expect(blocks).to.be.equal(1);
      }

      await instances[0].rpcClient.generate(2);
      await wait(5000);

      for (let i = 0; i < 3; i++) {
        const { result: blocks } = await instances[i].rpcClient.getBlockCount();
        expect(blocks).to.be.equal(3);
      }
    });
  });
});
