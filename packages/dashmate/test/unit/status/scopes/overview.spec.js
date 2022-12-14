const getOverviewScopeFactory = require('../../../../src/status/scopes/overview');
const MasternodeStateEnum = require('../../../../src/enums/masternodeState');
const DockerStatusEnum = require('../../../../src/enums/dockerStatus');
const ServiceStatusEnum = require('../../../../src/enums/serviceStatus');

describe('getOverviewScopeFactory', () => {
  describe('getOverviewScope', () => {
    let mockGetCoreScope;
    let mockGetPlatformScope;
    let mockGetMasternodeScope;

    let config;
    let getOverviewScope;

    beforeEach(async function it() {
      mockGetCoreScope = this.sinon.stub();
      mockGetMasternodeScope = this.sinon.stub();
      mockGetPlatformScope = this.sinon.stub();

      config = { get: this.sinon.stub(), toEnvs: this.sinon.stub() };

      getOverviewScope = getOverviewScopeFactory(mockGetCoreScope,
        mockGetMasternodeScope, mockGetPlatformScope);
    });

    it('should just work', async () => {
      const mockCoreScope = {
        version: 'v1.2.3',
        dockerStatus: DockerStatusEnum.running,
        serviceStatus: ServiceStatusEnum.up,
        blockHeight: 1337,
        verificationProgress: 1,
        sizeOnDisk: 1,
      };

      const mockMasternodeScope = {
        state: MasternodeStateEnum.READY,
        poSePenalty: 0,
        lastPaidHeight: 100,
        lastPaidTime: '23 days ago',
        paymentQueuePosition: null,
        nextPaymentTime: 'in 1 day',
        sentinelState: '',
        sentinelVersion: 'v1.2',
      };

      const mockPlatformScope = {
        coreIsSynced: true,
        tenderdash: {
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.up,
          version: null,
          catchingUp: null,
          lastBlockHeight: null,
          latestAppHash: null,
          peers: null,
          network: null,
        },
      };

      mockGetCoreScope.returns(mockCoreScope);
      mockGetPlatformScope.returns(mockPlatformScope);
      mockGetMasternodeScope.returns(mockMasternodeScope);

      const scope = await getOverviewScope(config);

      expect(scope.core.version).to.be.equal(mockCoreScope.version);
      expect(scope.core.dockerStatus).to.be.equal(mockCoreScope.dockerStatus);
      expect(scope.core.dockerStatus).to.be.equal(mockCoreScope.dockerStatus);
    });

    it('should not load if masternode or platform disabled ', async () => {
      config.get.withArgs('core.masternode.enable').returns(false);
      config.get.withArgs('network').returns('mainnet');

      const mockCoreScope = {
        version: 'v1.2.3',
        dockerStatus: DockerStatusEnum.running,
        serviceStatus: ServiceStatusEnum.up,
        blockHeight: 1337,
        verificationProgress: 1,
        sizeOnDisk: 1,
      };

      mockGetCoreScope.returns(mockCoreScope);

      const scope = await getOverviewScope(config);

      expect(mockGetMasternodeScope.notCalled).to.be.true();
      expect(scope.masternode.state).to.be.equal(null);
      expect(scope.masternode.proTxHash).to.be.equal(null);
      expect(scope.masternode.sentinel.version).to.be.equal(null);
      expect(scope.masternode.sentinel.state).to.be.equal(null);
      expect(scope.masternode.nodeState).to.be.equal(null);

      expect(mockGetPlatformScope.notCalled).to.be.true();
      expect(scope.platform.tenderdash).to.be.equal(null);
    });
  });
});
