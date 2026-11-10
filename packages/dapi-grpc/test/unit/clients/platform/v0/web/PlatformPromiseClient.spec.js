const { v0: { PlatformPromiseClient } } = require('../../../../../../browser');

describe('PlatformPromiseClient (web)', () => {
  let platformPromiseClient;
  let request;
  let response;
  let metadata;

  beforeEach(function main() {
    request = 'test request';
    response = 'test response';
    metadata = { key: 'value' };

    platformPromiseClient = new PlatformPromiseClient('https://localhost/');
    platformPromiseClient.client = {
      getOwnerTransactionsInBlock: this.sinon.stub().yields(null, response),
      getBlocks: this.sinon.stub().yields(null, response),
    };
  });

  describe('#getOwnerTransactionsInBlock', () => {
    it('should get the transactions of an owner in a block', async () => {
      const result = await platformPromiseClient.getOwnerTransactionsInBlock(request, metadata);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getOwnerTransactionsInBlock)
        .to.be.calledOnceWith(request, metadata);
    });

    it('should reject with the error the service answers', async () => {
      const error = new Error('unavailable');
      platformPromiseClient.client.getOwnerTransactionsInBlock.yields(error);

      await expect(platformPromiseClient.getOwnerTransactionsInBlock(request))
        .to.be.rejectedWith(error);
    });
  });

  describe('#getBlocks', () => {
    it('should get blocks', async () => {
      const result = await platformPromiseClient.getBlocks(request, metadata);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getBlocks)
        .to.be.calledOnceWith(request, metadata);
    });

    it('should reject with the error the service answers', async () => {
      const error = new Error('unavailable');
      platformPromiseClient.client.getBlocks.yields(error);

      await expect(platformPromiseClient.getBlocks(request)).to.be.rejectedWith(error);
    });
  });
});
