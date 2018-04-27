const Docker = require('dockerode');

const MongoDbInstanceOptions = require('../../../../../lib/test/services/mongoDb/MongoDbInstanceOptions');
const Network = require('../../../../../lib/test/services/docker/Network');

describe('Image', () => {
  it('should create a network according to options', async () => {
    const options = new MongoDbInstanceOptions();
    const { name, driver } = options.getContainerNetworkOptions();
    const network = new Network(name, driver);

    await network.create();

    const dockerNetwork = new Docker().getNetwork(name);
    const { Name, Driver } = await dockerNetwork.inspect();

    expect(Name).to.equal(name);
    expect(Driver).to.equal(driver);
  });

  it('should not fail creating a network that already exists', async () => {
    const options = new MongoDbInstanceOptions();
    const { name, driver } = options.getContainerNetworkOptions();
    const network = new Network(name, driver);

    await network.create();

    const dockerNetwork = new Docker().getNetwork(name);
    const { Name, Driver } = await dockerNetwork.inspect();

    expect(Name).to.equal(name);
    expect(Driver).to.equal(driver);
  });
});
