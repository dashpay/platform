const CorePromiseClient = require('../../../../clients/nodejs/CorePromiseClient');

describe('CorePromiseClient', () => {
  let corePromiseClient;
  let request;
  let response;

  beforeEach(function main() {
    request = 'test request';
    response = 'test response';

    corePromiseClient = new CorePromiseClient('localhost');
    corePromiseClient.client = {
      getStatus: this.sinon.stub().resolves(response),
      getBlock: this.sinon.stub().resolves(response),
      sendTransaction: this.sinon.stub().resolves(response),
      getTransaction: this.sinon.stub().resolves(response),
      getEstimatedTransactionFee: this.sinon.stub().resolves(response),
    };
  });

  describe('#getStatus', () => {
    it('should return status', async () => {
      const result = await corePromiseClient.getStatus(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.getStatus).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.getStatus({}, 'metadata');

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

  describe('#sendTransaction', () => {
    it('should send transaction', async () => {
      const result = await corePromiseClient.sendTransaction(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.sendTransaction).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.sendTransaction({}, 'metadata');

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
});
