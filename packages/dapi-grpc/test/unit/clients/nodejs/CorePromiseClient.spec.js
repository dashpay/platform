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
      getLastUserStateTransitionHash: this.sinon.stub().resolves(response),
      subscribeToBlockHeadersWithChainLocks: this.sinon.stub().resolves(response),
      updateState: this.sinon.stub().resolves(response),
    };
  });

  describe('#getLastUserStateTransitionHash', () => {
    it('should get last user state transition hash', async () => {
      const result = await corePromiseClient.getLastUserStateTransitionHash(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.getLastUserStateTransitionHash).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.getLastUserStateTransitionHash({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#subscribeToBlockHeadersWithChainLocks', () => {
    it('should subscribe to block headers with chain locks', async () => {
      const result = await corePromiseClient.subscribeToBlockHeadersWithChainLocks(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.subscribeToBlockHeadersWithChainLocks)
        .to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.subscribeToBlockHeadersWithChainLocks({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#updateState', () => {
    it('should update state', async () => {
      const result = await corePromiseClient.updateState(request);

      expect(result).to.equal(response);
      expect(corePromiseClient.client.updateState).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        corePromiseClient.updateState({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });
});
