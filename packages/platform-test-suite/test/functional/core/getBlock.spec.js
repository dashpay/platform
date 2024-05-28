const Dash = require('dash');

const createClientWithoutWallet = require('../../../lib/test/createClientWithoutWallet');

const { Core: { Block }, Essentials: { Buffer } } = Dash;

describe.skip('Core', () => {
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
        .getDAPIClient().core.getBlockchainStatus();

      const blockBinary = await client.getDAPIClient().core.getBlockByHeight(bestBlockHeight);

      expect(blockBinary).to.be.an.instanceof(Buffer);

      const block = new Block(blockBinary);
      expect(block).to.be.an.instanceOf(Block);
    });

    it('should throw NotFound error when the block by height was not found', async () => {
      try {
        await client.getDAPIClient()
          .core
          .getBlockByHeight(1000000000);

        expect.fail('should throw NotFound error');
      } catch (e) {
        expect(e.message).to.equal('Invalid block height');
        expect(e.code).to.equal(5);
      }
    });

    // TODO(core-20):
    //  restore test. Apparently something changed in the Core RPC API
    it.skip('should throw NotFound error when the block by hash was not found', async () => {
      try {
        await client.getDAPIClient().core.getBlockByHash('hash');

        expect.fail('should throw NotFound error');
      } catch (e) {
        expect(e.message).to.equal('Block not found');
        expect(e.code).to.equal(5);
      }
    });
  });
});
