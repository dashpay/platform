const { Block } = require('@dashevo/dashcore-lib');

const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

describe('Core', () => {
  describe('getBlock', () => {
    let client;

    before(() => {
      client = createClientWithoutWallet();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should get block by hash', async () => {
      const blockHash = await client.getDAPIClient().core.getBestBlockHash();

      const blockBinary = await client.getDAPIClient().core.getBlockByHash(blockHash);
      expect(blockBinary).to.be.an.instanceof(Buffer);

      const block = new Block(blockBinary);
      expect(block.hash).to.equal(blockHash);
    });

    it('should get block by height', async () => {
      const { chain: { blocksCount: bestBlockHeight } } = await client
        .getDAPIClient().core.getStatus();

      const blockBinary = await client.getDAPIClient().core.getBlockByHeight(bestBlockHeight);

      expect(blockBinary).to.be.an.instanceof(Buffer);

      const block = new Block(blockBinary);
      expect(block).to.be.an.instanceOf(Block);
    });

    it('should respond with an invalid argument error if the block by height was not found', async () => {
      let broadcastError;
      try {
        await client.getDAPIClient().core.getBlockByHeight(1000000000);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.equal('3 INVALID_ARGUMENT: Invalid block height');
      expect(broadcastError.code).to.equal(3);
    });

    it('should respond with null if the block by hash was not found', async () => {
      const block = await client.getDAPIClient().core.getBlockByHash('hash');

      expect(block).to.equal(null);
    });
  });
});
