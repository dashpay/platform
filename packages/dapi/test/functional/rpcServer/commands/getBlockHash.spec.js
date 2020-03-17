const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

describe('rpcServer', function main() {
  this.timeout(200000);

  describe('getBlockHash', () => {
    let removeDapi;
    let dapiClient;
    let blocksNumber;

    beforeEach(async () => {
      const {
        dapiCore,
        dashCore,
        remove,
      } = await startDapi();

      removeDapi = remove;

      dapiClient = dapiCore.getApi();
      const coreAPI = dashCore.getApi();

      const { result: addressString } = await coreAPI.getNewAddress();

      blocksNumber = 500;

      await coreAPI.generateToAddress(blocksNumber, addressString);
    });

    afterEach(async () => {
      await removeDapi();
    });

    it('should get block hash by height', async () => {
      const height = blocksNumber - 10;
      const hash = await dapiClient.getBlockHash(height);

      expect(hash).to.be.a('string');
    });

    it('should return RPC error if hash not found', async () => {
      const height = blocksNumber * 3;

      try {
        await dapiClient.getBlockHash(height);

        expect.fail('Should throw error');
      } catch (e) {
        expect(e.name).to.equal('RPCError');
        expect(e.message).contains('Block height out of range');
      }
    });
  });
});
