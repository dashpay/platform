const startIPFSInstance = require('../../../../../lib/test/services/IPFS/startIPFSInstance');

describe('startIPFSInstance', function main() {
  this.timeout(40000);

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
  });
});
