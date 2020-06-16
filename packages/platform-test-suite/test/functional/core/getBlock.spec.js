const { Block } = require('@dashevo/dashcore-lib');

describe.skip('Core', () => {
  describe('getBlock', () => {
    it('should get block by hash', async () => {
      const blockHash = await dashClient.clients.dapi.getBestBlockHash();

      const blockBinary = await dashClient.clients.dapi.getBlockByHash(blockHash);
      expect(blockBinary).to.be.an.instanceof(Buffer);
      const block = new Block(blockBinary);

      expect(block.hash).to.equal(blockHash);
    });

    it('should get block by height', async () => {
      const { blocks } = await dashClient.clients.dapi.getStatus();

      const blockBinary = await dashClient.clients.dapi.getBlockByHeight(blocks);

      expect(blockBinary).to.be.an.instanceof(Buffer);
      const block = new Block(blockBinary);

      expect(block).to.be.an.instanceOf(Block);
    });

    it('should respond with an invalid argument error if the block by height was not found', async () => {
      try {
        await dashClient.clients.dapi.getBlockByHeight(1000000000);

        expect.fail('Should throw an invalid argument error');
      } catch (e) {
        expect(e.message).to.equal('3 INVALID_ARGUMENT: Invalid block height');
        expect(e.code).to.equal(3);
      }
    });

    it('should respond with null if the block by hash was not found', async () => {
      const block = await dashClient.clients.dapi.getBlockByHash('hash');
      expect(block).to.equal(null);
    });
  });
});
