const Docker = require('dockerode');

const startDashCoreInstance = require('../../../../lib/test/services/dashCore/startDashCoreInstance');

async function wait(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function stopRunningContainers() {
  const docker = new Docker();
  const containers = await docker.listContainers();

  for (let i = 0; i < containers.length; i++) {
    const container = containers[i];
    await docker.getContainer(container.Id).stop();
  }
}

describe('startDashCoreInstance', function main() {
  this.timeout(40000);

  before(async () => stopRunningContainers());

  describe('One instance', () => {
    let instance;

    before(async () => {
      instance = await startDashCoreInstance();
    });

    it('should has container running', async () => {
      const { State } = await instance.container.inspect();
      expect(State.Status).to.equal('running');
    });

    it('should has RPC connected', async () => {
      const { result } = await instance.rpcClient.getInfo();
      expect(result.version).to.equal(120300);
    });
  });

  describe('Three instances', () => {
    let instances;

    before(async () => {
      instances = await startDashCoreInstance.many(3);
    });

    it('should have containers running', async () => {
      for (let i = 0; i < 3; i++) {
        const { State } = await instances[i].container.inspect();
        expect(State.Status).to.equal('running');
      }
    });

    it('should propagate blocks between instances', async () => {
      for (let i = 0; i < 3; i++) {
        const { result: blocks } = await instances[i].rpcClient.getBlockCount();
        expect(blocks).to.equal(0);
      }

      await instances[0].rpcClient.generate(2);
      await wait(10000);

      for (let i = 0; i < 3; i++) {
        const { result: blocks } = await instances[i].rpcClient.getBlockCount();
        expect(blocks).to.equal(2);
      }
    });
  });
});
