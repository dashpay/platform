const getCoreDefinition = require('../../lib/getCoreDefinition');

describe('getCoreDefinition', () => {
  describe('v0', () => {
    it('should return loaded GRPC package definition', async () => {
      const coreDefinition = getCoreDefinition(0);

      expect(coreDefinition).to.be.an('function');
      expect(coreDefinition).to.have.property('service');

      expect(coreDefinition.service).to.have.property('broadcastTransaction');
      expect(coreDefinition.service.broadcastTransaction.path).to.equal('/org.dash.platform.dapi.v0.Core/broadcastTransaction');

      expect(coreDefinition.service).to.have.property('getTransaction');
      expect(coreDefinition.service.getTransaction.path).to.equal('/org.dash.platform.dapi.v0.Core/getTransaction');

      expect(coreDefinition.service).to.have.property('getBlockchainStatus');
      expect(coreDefinition.service.getBlockchainStatus.path).to.equal('/org.dash.platform.dapi.v0.Core/getBlockchainStatus');

      expect(coreDefinition.service).to.have.property('getMasternodeStatus');
      expect(coreDefinition.service.getMasternodeStatus.path).to.equal('/org.dash.platform.dapi.v0.Core/getMasternodeStatus');

      expect(coreDefinition.service).to.have.property('getBlock');
      expect(coreDefinition.service.getBlock.path).to.equal('/org.dash.platform.dapi.v0.Core/getBlock');

      expect(coreDefinition.service).to.have.property('getEstimatedTransactionFee');
      expect(coreDefinition.service.getEstimatedTransactionFee.path).to.equal('/org.dash.platform.dapi.v0.Core/getEstimatedTransactionFee');

      expect(coreDefinition.service).to.have.property('subscribeToBlockHeadersWithChainLocks');
      expect(coreDefinition.service.subscribeToBlockHeadersWithChainLocks.path).to.equal('/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks');

      expect(coreDefinition.service).to.have.property('subscribeToTransactionsWithProofs');
      expect(coreDefinition.service.subscribeToTransactionsWithProofs.path).to.equal('/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs');
    });
  });
});
