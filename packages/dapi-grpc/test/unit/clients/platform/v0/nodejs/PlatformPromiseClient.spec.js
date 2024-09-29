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
      getIdentitiesContractKeys: this.sinon.stub().resolves(response),
      getDataContract: this.sinon.stub().resolves(response),
      getDocuments: this.sinon.stub().resolves(response),
      getEpochsInfo: this.sinon.stub().resolves(response),
      getProtocolVersionUpgradeVoteStatus: this.sinon.stub().resolves(response),
      getProtocolVersionUpgradeState: this.sinon.stub().resolves(response),
      getIdentityContractNonce: this.sinon.stub().resolves(response),
      getIdentityNonce: this.sinon.stub().resolves(response),
      getIdentityKeys: this.sinon.stub().resolves(response),
      getIdentityBalance: this.sinon.stub().resolves(response),
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

  describe('#getIdentitiesContractKeys', () => {
    it('should get identities', async () => {
      const result = await platformPromiseClient.getIdentitiesContractKeys(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getIdentitiesContractKeys)
        .to.be.calledOnceWith(request);
    });

    it('should throw an error when metadata is not an object', async () => {
      try {
        platformPromiseClient.getIdentitiesContractKeys({}, 'metadata');

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

  describe('#getProtocolVersionUpgradeVoteStatus', () => {
    it('should get version upgrade votes status', async () => {
      const result = await platformPromiseClient.getProtocolVersionUpgradeVoteStatus(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getProtocolVersionUpgradeVoteStatus)
        .to.be.calledOnceWith(request);
    });
  });

  describe('#getProtocolVersionUpgradeState', () => {
    it('should get version upgrade state', async () => {
      const result = await platformPromiseClient.getProtocolVersionUpgradeState(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getProtocolVersionUpgradeState)
        .to.be.calledOnceWith(request);
    });
  });

  describe('#getIdentityContractNonce', () => {
    it('should get identity contract nonce', async () => {
      const result = await platformPromiseClient.getIdentityContractNonce(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getIdentityContractNonce)
        .to.be.calledOnceWith(request);
    });
  });

  describe('#getIdentityNonce', () => {
    it('should get identity nonce', async () => {
      const result = await platformPromiseClient.getIdentityNonce(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getIdentityNonce)
        .to.be.calledOnceWith(request);
    });
  });

  describe('#getIdentityKeys', () => {
    it('should get identity keys', async () => {
      const result = await platformPromiseClient.getIdentityKeys(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getIdentityKeys)
        .to.be.calledOnceWith(request);
    });
  });

  describe('#getIdentityBalance', () => {
    it('should get identity balance', async () => {
      const result = await platformPromiseClient.getIdentityBalance(request);

      expect(result).to.equal(response);
      expect(platformPromiseClient.client.getIdentityBalance)
        .to.be.calledOnceWith(request);
    });
  });
});
