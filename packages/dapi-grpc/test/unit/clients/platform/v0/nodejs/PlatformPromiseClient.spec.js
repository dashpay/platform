const { v0: { PlatformPromiseClient } } = require('../../../../../..');

describe('PlatformPromiseClient', () => {
  let platformPromiseClient;
  let request;
  let response;

  beforeEach(function main() {
    request = 'test request';
    response = 'test response';

    platformPromiseClient = new PlatformPromiseClient('https://localhost/');
    platformPromiseClient.client = {
      broadcastStateTransition: this.sinon.stub().resolves(response),
      getIdentity: this.sinon.stub().resolves(response),
      getDataContract: this.sinon.stub().resolves(response),
      getDocuments: this.sinon.stub().resolves(response),
      getEpochsInfo: this.sinon.stub().resolves(response),
      getVersionUpgradeVoteStatus: this.sinon.stub().resolves(response),
      getVersionUpgradeState: this.sinon.stub().resolves(response),
    };
  });

  describe('#broadcastStateTransition', () => {
    it('should broadcast state transition', async () => {
      const result = await platformPromiseClient.broadcastStateTransition(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.broadcastStateTransition).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        platformPromiseClient.broadcastStateTransition({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#getIdentity', () => {
    it('should get identity', async () => {
      const result = await platformPromiseClient.getIdentity(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getIdentity)
        .to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        platformPromiseClient.getIdentity({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#getDataContract', () => {
    it('should get data contract', async () => {
      const result = await platformPromiseClient.getDataContract(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getDataContract).to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        platformPromiseClient.getDataContract({}, 'metadata');

        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.message).to.equal('metadata must be an object');
      }
    });
  });

  describe('#getDocuments', () => {
    it('should get documents', async () => {
      const result = await platformPromiseClient.getDocuments(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getDocuments).to.be.calledOnceWith(request);
    });
  });

  describe('#getEpochsInfo', () => {
    it('should get epochs info', async () => {
      const result = await platformPromiseClient.getEpochsInfo(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getEpochsInfo).to.be.calledOnceWith(request);
    });
  });

  describe('#getVersionUpgradeVoteStatus', () => {
    it('should get version upgrade vote status', async () => {
      const result = await platformPromiseClient.getVersionUpgradeVoteStatus(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getVersionUpgradeVoteStatus)
        .to.be.calledOnceWith(request);
    });
  });

  describe('#getVersionUpgradeState', () => {
    it('should get version upgrade state', async () => {
      const result = await platformPromiseClient.getVersionUpgradeState(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getVersionUpgradeState).to.be.calledOnceWith(request);
    });
  });
});
