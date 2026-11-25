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
      getRecentStateTransitionStatus: this.sinon.stub().yields(null, response),
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

  describe('#getRecentStateTransitionStatus', () => {
    it('should get the node\'s status of a state transition', async () => {
      const result = await platformPromiseClient.getRecentStateTransitionStatus(request, metadata);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getRecentStateTransitionStatus)
        .to.be.calledOnceWith(request, metadata);
    });

    it('should reject with the error the service answers', async () => {
      const error = new Error('unavailable');
      platformPromiseClient.client.getRecentStateTransitionStatus.yields(error);

      await expect(platformPromiseClient.getRecentStateTransitionStatus(request))
        .to.be.rejectedWith(error);
    });
  });

  describe('#subscribeToStateTransitions', () => {
    let stream;
    let handlers;

    beforeEach(function beforeEach() {
      handlers = {};
      stream = {
        on: this.sinon.spy((type, handler) => {
          handlers[type] = handler;
        }),
      };
      platformPromiseClient.client.subscribeToStateTransitions = this.sinon.stub()
        .returns(stream);
    });

    it('should return the stream of the service without waiting for it', () => {
      const result = platformPromiseClient.subscribeToStateTransitions(request, metadata);

      expect(result).to.equal(stream);
      expect(platformPromiseClient.client.subscribeToStateTransitions)
        .to.be.calledOnceWith(request, metadata);
    });

    it('should report a stream that ends with a failed status as an error', () => {
      const errors = [];
      platformPromiseClient.subscribeToStateTransitions(request)
        .on('error', (error) => errors.push(error));

      handlers.end({ code: 14, details: 'unavailable', metadata: { key: 'value' } });
      handlers.end({ code: 0, details: 'ok', metadata: {} });

      expect(errors).to.have.lengthOf(1);
      expect(errors[0].code).to.equal(14);
      expect(errors[0].message).to.equal('unavailable');
      expect(errors[0].metadata).to.deep.equal({ key: 'value' });
    });

    it('should pass the events of the stream through', () => {
      const messages = [];
      platformPromiseClient.subscribeToStateTransitions(request)
        .on('data', (message) => messages.push(message));

      handlers.data(response);

      expect(messages).to.deep.equal([response]);
    });
  });
});
