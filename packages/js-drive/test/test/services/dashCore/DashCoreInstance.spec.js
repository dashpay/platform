const Docker = require('dockerode');

const DashCoreInstance = require('../../../../lib/test/services/dashCore/DashCoreInstance');

async function wait(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

describe('DashCoreInstance', function main() {
  this.timeout(40000);

  describe('before start', () => {
    const instance = new DashCoreInstance();

    it('should throw an error if connect', async () => {
      const instanceTwo = new DashCoreInstance();

      let error;
      try {
        await instance.connect(instanceTwo);
      } catch (err) {
        error = err;
      }
      expect(error.message).to.equal('Instance should be started before!');
    });

    it('should not crash if stop', async () => {
      await instance.stop();
    });

    it('should not crash if clean', async () => {
      await instance.clean();
    });

    it('should return null if getIp', () => {
      const ip = instance.getIp();
      expect(ip).to.be.null();
    });

    it('should return null if getAddress', () => {
      const address = instance.getAddress();
      expect(address).to.be.null();
    });

    it('should return empty object if getApi', () => {
      const api = instance.getApi();
      expect(api).to.deep.equal({});
    });

    it('should return empty object if getZmqSocket', () => {
      const config = instance.getZmqSockets();
      expect(config).to.deep.equal({});
    });
  });

  describe('usage', () => {
    const instance = new DashCoreInstance();

    after(async () => instance.clean());

    it('should start an instance with a bridge dash_test_network', async () => {
      await instance.start();
      const network = new Docker().getNetwork('dash_test_network');
      const { Driver } = await network.inspect();
      const { NetworkSettings: { Networks } } = await instance.container.inspect();
      const networks = Object.keys(Networks);
      expect(Driver).to.equal('bridge');
      expect(networks.length).to.equal(1);
      expect(networks[0]).to.equal('dash_test_network');
    });

    it('should start an instance with the default options', async () => {
      await instance.start();
      const { Args } = await instance.container.inspect();
      expect(Args).to.deep.equal([
        `-port=${instance.options.CORE.port}`,
        `-rpcuser=${instance.options.RPC.user}`,
        `-rpcpassword=${instance.options.RPC.password}`,
        '-rpcallowip=0.0.0.0/0',
        '-regtest=1',
        `-rpcport=${instance.options.RPC.port}`,
        `-zmqpubhashblock=tcp://0.0.0.0:${instance.options.ZMQ.port}`,
      ]);
    });

    it('should not crash if start is called multiple times', async () => {
      await instance.start();
      await instance.start();
    });

    it('should stop the instance', async () => {
      await instance.stop();
      const { State } = await instance.container.inspect();
      expect(State.Status).to.equal('exited');
    });

    it('should start after stop', async () => {
      await instance.start();
      const { State } = await instance.container.inspect();
      expect(State.Status).to.equal('running');
    });

    it('should return ZMQ sockets configuration', () => {
      const zmqPort = instance.options.ZMQ.port;
      const zmqSockets = instance.getZmqSockets();
      expect(zmqSockets).to.deep.equal({
        hashblock: `tcp://127.0.0.1:${zmqPort}`,
      });
    });

    it('should return RPC client', () => {
      const rpcPort = instance.options.RPC.port;
      const rpcClient = instance.getApi();
      expect(rpcClient.host).to.be.equal('127.0.0.1');
      expect(rpcClient.port).to.be.equal(rpcPort);
    });

    it('should return container IP', () => {
      expect(instance.getIp()).to.be.equal(instance.containerIp);
    });

    it('should clean the instance', async () => {
      await instance.clean();

      let error;
      try {
        await instance.container.inspect();
      } catch (err) {
        error = err;
      }
      expect(error.statusCode).to.equal(404);
      expect(error.reason).to.equal('no such container');
    });
  });

  describe('containers removal', () => {
    const instanceOne = new DashCoreInstance();
    const instanceTwo = new DashCoreInstance();
    const instanceThree = new DashCoreInstance();
    let sandbox;

    before(function before() {
      sandbox = this.sinon;
    });
    after(async () => {
      await Promise.all([
        instanceOne.clean(),
        instanceTwo.clean(),
        instanceThree.clean(),
      ]);
    });

    it('should call createContainer only once when start/stop/start', async () => {
      const createContainerSpy = sandbox.spy(instanceOne, 'createContainer');

      await instanceOne.start();
      await instanceOne.stop();
      await instanceOne.start();

      expect(createContainerSpy.callCount).to.equal(1);
    });

    it('should remove container if port if busy before creating a new one', async () => {
      instanceTwo.options = instanceOne.options;
      instanceThree.options = instanceOne.options;
      const removeContainerSpy = sandbox.spy(instanceThree, 'removeContainer');

      await instanceOne.start();
      await instanceTwo.start();
      await instanceThree.start();

      expect(removeContainerSpy.callCount).to.be.equal(1);
    });
  });

  describe('networking', () => {
    const instanceOne = new DashCoreInstance();
    const instanceTwo = new DashCoreInstance();

    before(async () => {
      await Promise.all([
        instanceOne.start(),
        instanceTwo.start(),
      ]);
    });
    after(async () => {
      await Promise.all([
        instanceOne.clean(),
        instanceTwo.clean(),
      ]);
    });

    it('should be connected each other', async () => {
      await instanceOne.connect(instanceTwo);
      await wait(2000);

      const { result: peersInstanceOne } = await instanceOne.rpcClient.getPeerInfo();
      const { result: peersInstanceTwo } = await instanceTwo.rpcClient.getPeerInfo();
      const peerInstanceOneIp = peersInstanceOne[0].addr.split(':')[0];
      const peerInstanceTwoIp = peersInstanceTwo[0].addr.split(':')[0];

      expect(peersInstanceOne.length).to.equal(1);
      expect(peersInstanceTwo.length).to.equal(1);
      expect(peerInstanceOneIp).to.equal(instanceTwo.getIp());
      expect(peerInstanceTwoIp).to.equal(instanceOne.getIp());
    });

    it('should propagate blocks from one instance to the other', async () => {
      const { result: blocksInstanceOne } = await instanceOne.rpcClient.getBlockCount();
      const { result: blocksInstanceTwo } = await instanceTwo.rpcClient.getBlockCount();
      expect(blocksInstanceOne).to.equal(0);
      expect(blocksInstanceTwo).to.equal(0);

      await instanceOne.rpcClient.generate(2);
      await wait(2000);

      const { result: blocksOne } = await instanceOne.rpcClient.getBlockCount();
      const { result: blocksTwo } = await instanceTwo.rpcClient.getBlockCount();
      expect(blocksOne).to.equal(2);
      expect(blocksTwo).to.equal(2);
    });
  });

  describe('ports', () => {
    const instanceOne = new DashCoreInstance();
    const instanceTwo = new DashCoreInstance();
    const instanceThree = new DashCoreInstance();

    let sandbox;

    before(function before() {
      sandbox = this.sinon;
    });
    after(async () => {
      await Promise.all([
        instanceOne.clean(),
        instanceTwo.clean(),
        instanceThree.clean(),
      ]);
    });

    it('should retry start container with another port if it is busy', async () => {
      instanceTwo.options = instanceOne.options;
      instanceThree.options = instanceOne.options;
      const instanceThreeSpy = sandbox.spy(instanceThree, 'createContainer');

      await instanceOne.start();
      await instanceTwo.start();
      await instanceThree.start();

      expect(instanceThreeSpy.callCount).to.be.equal(2);
    });
  });

  describe('RPC', () => {
    const instance = new DashCoreInstance();

    after(async () => instance.clean());

    it('should work after starting the instance', async () => {
      await instance.start();

      const rpcClient = instance.getApi();
      const { result } = await rpcClient.getInfo();
      expect(result.version).to.equal(120300);
    });

    it('should work after restarting the instance', async () => {
      await instance.start();
      await instance.stop();
      await instance.start();

      const rpcClient = instance.getApi();
      const { result } = await rpcClient.getInfo();
      expect(result.version).to.equal(120300);
    });
  });
});
