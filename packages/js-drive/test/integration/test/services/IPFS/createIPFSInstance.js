const Docker = require('dockerode');
const createIPFSInstance = require('../../../../../lib/test/services/IPFS/createIPFSInstance');

describe('createIPFSInstance', function main() {
  this.timeout(40000);

  describe('usage', () => {
    let instance;

    before(async () => {
      instance = await createIPFSInstance();
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
        '--',
        '/usr/local/bin/start_ipfs',
        'daemon',
      ]);
    });

    it('should get IPFS address', async () => {
      await instance.start();

      const address = instance.getIpfsAddress();

      expect(address).to.equal(`/ip4/${instance.getIp()}/tcp/${instance.options.getIpfsInternalPort()}`);
    });

    it('should get IPFS client', async () => {
      await instance.start();

      const client = instance.getApi();
      await client.repo.stat();
    });
  });

  describe('networking', () => {
    let instanceOne;
    let instanceTwo;

    before(async () => {
      instanceOne = await createIPFSInstance();
      instanceTwo = await createIPFSInstance();
    });
    before(async () => {
      await Promise.all([
        instanceOne.start(),
        instanceTwo.start(),
      ]);
    });
    after(async () => {
      await Promise.all([
        instanceOne.remove(),
        instanceTwo.remove(),
      ]);
    });

    it('should propagate data from one instance to the other', async () => {
      const clientOne = instanceOne.getApi();
      const cid = await clientOne.dag.put({ name: 'world' }, { format: 'dag-cbor', hashAlg: 'sha2-256' });

      const clientTwo = instanceTwo.getApi();
      const data = await clientTwo.dag.get(cid, 'name', { format: 'dag-cbor', hashAlg: 'sha2-256' });

      expect(data.value).to.equal('world');
    });
  });
});
