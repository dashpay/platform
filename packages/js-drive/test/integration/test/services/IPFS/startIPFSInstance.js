const removeContainers = require('../../../../../lib/test/services/docker/removeContainers');
const startIPFSInstance = require('../../../../../lib/test/services/IPFS/startIPFSInstance');

describe('startIPFSInstance', function main() {
  this.timeout(40000);

  before(removeContainers);

  describe('One instance', () => {
    let instance;
    before(async () => {
      instance = await startIPFSInstance();
    });
    after(async () => instance.remove());

    it('should start one instance', async () => {
      const client = instance.getApi();
      const actualTrueObject = await client.block.put(Buffer.from('{"true": true}'));
      const expectedTrueObject = await client.block.get(actualTrueObject.cid);
      expect(expectedTrueObject.data).to.be.deep.equal(actualTrueObject.data);
    });
  });

  describe('Three instances', () => {
    let instances;
    before(async () => {
      instances = await startIPFSInstance.many(3);
    });
    after(async () => {
      const promises = instances.map(instance => instance.remove());
      await Promise.all(promises);
    });

    it('should start many instances', async () => {
      const clientOne = await instances[0].getApi();
      const actualTrueObject = await clientOne.block.put(Buffer.from('{"true": true}'));

      for (let i = 1; i < 3; i++) {
        const client = await instances[i].getApi();
        const expectedTrueObject = await client.block.get(actualTrueObject.cid);
        expect(expectedTrueObject.data).to.be.deep.equal(actualTrueObject.data);
      }
    });

    it('should propagate data between instances', async () => {
      const clientOne = instances[0].getApi();
      const cid = await clientOne.dag.put({ name: 'world' }, { format: 'dag-cbor', hashAlg: 'sha2-256' });

      for (let i = 0; i < 3; i++) {
        const ipfs = instances[i].getApi();
        const data = await ipfs.dag.get(cid, 'name', { format: 'dag-cbor', hashAlg: 'sha2-256' });
        expect(data.value).to.equal('world');
      }
    });
  });
});
