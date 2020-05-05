describe('Core', () => {
  describe('getBlockHash', () => {
    let lastBlockHeight;

    before(async () => {
      ({ blocks: lastBlockHeight } = await dashClient.clients.dapi.getStatus());
    });

    it('should get block hash by height', async () => {
      const height = lastBlockHeight - 10;
      const hash = await dashClient.clients.dapi.getBlockHash(height);

      expect(hash).to.be.a('string');
    });

    it('should return RPC error if hash not found', async () => {
      const height = lastBlockHeight * 2;

      try {
        await dashClient.clients.dapi.getBlockHash(height);

        expect.fail('Should throw error');
      } catch (e) {
        expect(e.name).to.equal('RPCError');
        expect(e.message).contains('Block height out of range');
      }
    });
  });
});
