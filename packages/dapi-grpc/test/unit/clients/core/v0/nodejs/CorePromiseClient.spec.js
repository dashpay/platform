const { v0: { CorePromiseClient } } = require('../../../../../..');

describe('CorePromiseClient', () => {
  let corePromiseClient;
  let request;
  let response;

  beforeEach(function main() {
    request = 'test request';
    response = 'test response';

    corePromiseClient = new CorePromiseClient('https://localhost/');
    corePromiseClient.client = {
      getBlockchainStatus: this.sinon.stub().resolves(response),
      getMasternodeStatus: this.sinon.stub().resolves(response),
      getBlock: this.sinon.stub().resolves(response),
      broadcastTransaction: this.sinon.stub().resolves(response),
      getTransaction: this.sinon.stub().resolves(response),
      getEstimatedTransactionFee: this.sinon.stub().resolves(response),
      subscribeToTransactionsWithProofs: this.sinon.stub().resolves(response),
    };
  });

  describe('#getBlockchainStatus', () => {
    it('should return core chain status', async () => {
      const result = await corePromiseClient.getBlockchainStatus(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.getBlockchainStatus).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.getBlockchainStatus({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#getMasternodeStatus', () => {
    it('should return masternode status', async () => {
      const result = await corePromiseClient.getMasternodeStatus(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.getMasternodeStatus)
        .to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.getMasternodeStatus({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#getBlock', () => {
    it('should get block', async () => {
      const result = await corePromiseClient.getBlock(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.getBlock).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.getBlock({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#broadcastTransaction', () => {
    it('should broadcast transaction', async () => {
      const result = await corePromiseClient.broadcastTransaction(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.broadcastTransaction).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.broadcastTransaction({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#getTransaction', () => {
    it('should get transaction', async () => {
      const result = await corePromiseClient.getTransaction(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.getTransaction)
        .to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.getTransaction({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#getEstimatedTransactionFee', () => {
    it('should return status', async () => {
      const result = await corePromiseClient.getEstimatedTransactionFee(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.getEstimatedTransactionFee).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.getEstimatedTransactionFee({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#subscribeToTransactionsWithProofs', () => {
    it('should subscribe to transactions with proofs', async () => {
      const result = await corePromiseClient
        .subscribeToTransactionsWithProofs(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.subscribeToTransactionsWithProofs)
        .to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.subscribeToTransactionsWithProofs({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });
});
