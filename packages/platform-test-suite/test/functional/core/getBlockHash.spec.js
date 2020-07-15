const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Core', () => {
  describe('getBlockHash', () => {
    let client;
    let lastBlockHeight;

    before(async () => {
      client = createClientWithoutWallet();

      ({ blocks: lastBlockHeight } = await client.getDAPIClient().core.getStatus());
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should get block hash by height', async () => {
      const height = lastBlockHeight - 10;
      const hash = await client.getDAPIClient().core.getBlockHash(height);

      expect(hash).to.be.a('string');
    });

    it('should return RPC error if hash not found', async () => {
      const height = lastBlockHeight * 2;

      try {
        await client.getDAPIClient().core.getBlockHash(height);

        expect.fail('Should throw error');
      } catch (e) {
        expect(e.name).to.equal('JsonRpcError');
        expect(e.message).contains('Block height out of range');
      }
    });
  });
});
