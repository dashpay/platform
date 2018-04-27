const Docker = require('dockerode');

const DashCoreInstanceOptions = require('../../../../../lib/test/services/dashCore/DashCoreInstanceOptions');
const Network = require('../../../../../lib/test/services/docker/Network');
const getAwsEcrAuthorizationToken = require('../../../../../lib/test/services/docker/getAwsEcrAuthorizationToken');
const Image = require('../../../../../lib/test/services/docker/Image');
const Container = require('../../../../../lib/test/services/docker/Container');
const DockerInstance = require('../../../../../lib/test/services/docker/DockerInstance');

async function createInstance(options) {
  const { name: networkName, driver } = options.getContainerNetworkOptions();
  const imageName = options.getContainerImageName();
  const containerOptions = options.getContainerOptions();
  const network = new Network(networkName, driver);
  const authorizationToken = await getAwsEcrAuthorizationToken(process.env.AWS_DEFAULT_REGION);
  const image = new Image(imageName, authorizationToken);
  const container = new Container(networkName, imageName, containerOptions);
  return new DockerInstance(network, image, container, options);
}

describe('DockerInstance', function main() {
  this.timeout(40000);

  const options = new DashCoreInstanceOptions();

  describe('usage', () => {
    let instance;

    before(async () => {
      instance = await createInstance(options);
    });
    after(async () => instance.clean());

    it('should start a DockerInstance with DashCoreInstanceOptions network options', async () => {
      await instance.start();
      const { name, driver } = options.getContainerNetworkOptions();
      const dockerNetwork = new Docker().getNetwork(name);
      const { Driver } = await dockerNetwork.inspect();
      const { NetworkSettings: { Networks } } = await instance.container.details();
      const networks = Object.keys(Networks);
      expect(Driver).to.equal(driver);
      expect(networks.length).to.equal(1);
      expect(networks[0]).to.equal(name);
    });

    it('should start an instance with the DashCoreInstanceOptions options', async () => {
      await instance.start();
      const { Args } = await instance.container.details();
      expect(Args).to.deep.equal([
        `-port=${options.getDashdPort()}`,
        `-rpcuser=${options.getRpcUser()}`,
        `-rpcpassword=${options.getRpcPassword()}`,
        '-rpcallowip=0.0.0.0/0',
        '-regtest=1',
        `-rpcport=${options.getRpcPort()}`,
        `-zmqpubhashblock=${options.getZmqSockets().hashblock}`,
      ]);
    });

    it('should not crash if start is called multiple times', async () => {
      await instance.start();
      await instance.start();
    });

    it('should stop the instance', async () => {
      await instance.stop();
      const { State } = await instance.container.details();
      expect(State.Status).to.equal('exited');
    });

    it('should start after stop', async () => {
      await instance.start();
      const { State } = await instance.container.details();
      expect(State.Status).to.equal('running');
    });

    it('should return instance IP', () => {
      expect(instance.getIp()).to.be.equal(instance.getIp());
    });

    it('should clean the instance', async () => {
      await instance.clean();

      let error;
      try {
        await instance.container.details();
      } catch (err) {
        error = err;
      }
      expect(error.statusCode).to.equal(404);
      expect(error.reason).to.equal('no such container');
    });
  });

  describe('ports', () => {
    let instanceOne;
    let instanceTwo;
    let instanceThree;
    let sandbox;

    before(async () => {
      instanceOne = await createInstance(new DashCoreInstanceOptions());
      instanceTwo = await createInstance(new DashCoreInstanceOptions());
      instanceThree = await createInstance(new DashCoreInstanceOptions());
    });
    beforeEach(function before() {
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
      instanceOne.container.ports = [4444];
      instanceTwo.container.ports = [4444];
      instanceThree.container.ports = [4444];
      const instanceThreeSpy = sandbox.spy(instanceThree, 'start');

      await instanceOne.start();
      await instanceTwo.start();
      await instanceThree.start();

      expect(instanceThreeSpy.callCount).to.be.above(0);
    });
  });
});
