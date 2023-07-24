const MasternodeSyncAssetEnum = require('../../../../src/status/enums/masternodeSyncAsset');
const ServiceStatusEnum = require('../../../../src/status/enums/serviceStatus');
const DockerStatusEnum = require('../../../../src/status/enums/dockerStatus');
const getCoreScopeFactory = require('../../../../src/status/scopes/core');
const determineStatus = require('../../../../src/status/determineStatus');
const providers = require('../../../../src/status/providers');
const PortStateEnum = require('../../../../src/status/enums/portState');
const getConfigMock = require('../../../../src/test/mock/getConfigMock');

describe('getCoreScopeFactory', () => {
  describe('#getCoreScope', () => {
    let mockRpcClient;
    let mockCreateRpcClient;
    let mockDockerCompose;
    let mockDetermineDockerStatus;
    let mockGithubProvider;
    let mockMNOWatchProvider;
    let mockInsightProvider;
    let mockGetConnectionHost;

    let network;
    let p2pService;
    let rpcService;

    let config;
    let getCoreScope;

    beforeEach(async function it() {
      mockRpcClient = {
        mnsync: this.sinon.stub(),
        getBlockchainInfo: this.sinon.stub(),
        getNetworkInfo: this.sinon.stub(),
      };
      mockCreateRpcClient = () => mockRpcClient;
      mockDockerCompose = { isNodeRunning: this.sinon.stub(), isServiceRunning: this.sinon.stub() };
      mockDetermineDockerStatus = this.sinon.stub(determineStatus, 'docker');
      mockGithubProvider = this.sinon.stub(providers.github, 'release');
      mockMNOWatchProvider = this.sinon.stub(providers.mnowatch, 'checkPortStatus');
      mockInsightProvider = this.sinon.stub(providers, 'insight');
      mockGetConnectionHost = this.sinon.stub();

      config = getConfigMock(this.sinon);

      config.get.withArgs('network').returns('testnet');
      config.get.withArgs('core.rpc.port').returns('8080');
      config.get.withArgs('core.p2p.port').returns('8081');
      config.get.withArgs('externalIp').returns('127.0.0.1');

      network = config.get('network');
      rpcService = `127.0.0.1:${config.get('core.rpc.port')}`;
      p2pService = `${config.get('externalIp')}:${config.get('core.p2p.port')}`;

      getCoreScope = getCoreScopeFactory(
        mockDockerCompose,
        mockCreateRpcClient,
        mockGetConnectionHost,
      );
    });

    it('should just work', async function it() {
      mockDockerCompose.isServiceRunning.resolves(true);
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);

      mockRpcClient.mnsync.returns({
        result:
          { AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED },
      });
      mockRpcClient.getNetworkInfo.returns({ result: { subversion: '/Dash Core:0.17.0.3/', connections: 1 } });
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          difficulty: 1,
          blocks: 2,
          headers: 3,
          chain: 'test',
          size_on_disk: 1337,
          verificationprogress: 1,
        },
      });

      mockGithubProvider.returns('v1337-dev');
      mockMNOWatchProvider.returns('OPEN');
      mockInsightProvider.returns({
        status: this.sinon.stub().returns({
          info: { blocks: 1337 },
        }),
      });

      const scope = await getCoreScope(config);

      const expectedScope = {
        network,
        p2pService: '127.0.0.1:8081',
        rpcService: '127.0.0.1:8080',
        version: '0.17.0.3',
        chain: 'test',
        latestVersion: 'v1337-dev',
        dockerStatus: DockerStatusEnum.running,
        serviceStatus: ServiceStatusEnum.up,
        peersCount: 1,
        p2pPortState: 'OPEN',
        blockHeight: 2,
        remoteBlockHeight: 1337,
        headerHeight: 3,
        difficulty: 1,
        verificationProgress: 1,
        sizeOnDisk: 1337,
        syncAsset: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
      };

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return status stopped if no service is running', async () => {
      mockDockerCompose.isNodeRunning.resolves(false);

      const scope = await getCoreScope(config);

      const expectedScope = {
        network,
        p2pService,
        rpcService,
        version: null,
        chain: null,
        latestVersion: null,
        dockerStatus: DockerStatusEnum.not_started,
        serviceStatus: ServiceStatusEnum.stopped,
        peersCount: null,
        p2pPortState: null,
        blockHeight: null,
        remoteBlockHeight: null,
        headerHeight: null,
        difficulty: null,
        verificationProgress: null,
        sizeOnDisk: null,
        syncAsset: null,
      };

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return status error if docker is not in running state', async () => {
      mockDockerCompose.isServiceRunning.resolves(true);
      mockDetermineDockerStatus.returns(DockerStatusEnum.restarting);

      const scope = await getCoreScope(config);

      const expectedScope = {
        network,
        p2pService,
        rpcService,
        version: null,
        chain: null,
        latestVersion: null,
        dockerStatus: DockerStatusEnum.restarting,
        serviceStatus: ServiceStatusEnum.error,
        peersCount: null,
        p2pPortState: null,
        blockHeight: null,
        remoteBlockHeight: null,
        headerHeight: null,
        difficulty: null,
        verificationProgress: null,
        sizeOnDisk: null,
        syncAsset: null,
      };

      expect(scope).to.be.deep.equal(expectedScope);
    });

    it('should not make any requests if docker status is bad', async function it() {
      mockDockerCompose.isServiceRunning.resolves(true);
      mockDetermineDockerStatus.returns(DockerStatusEnum.restarting);
      mockInsightProvider.returns({ status: this.sinon.stub() });

      const scope = await getCoreScope(config);

      const expectedScope = {
        network,
        p2pService,
        rpcService,
        version: null,
        chain: null,
        latestVersion: null,
        dockerStatus: DockerStatusEnum.restarting,
        serviceStatus: ServiceStatusEnum.error,
        peersCount: null,
        p2pPortState: null,
        blockHeight: null,
        remoteBlockHeight: null,
        headerHeight: null,
        difficulty: null,
        verificationProgress: null,
        sizeOnDisk: null,
        syncAsset: null,
      };

      expect(scope).to.be.deep.equal(expectedScope);

      expect(mockRpcClient.mnsync.notCalled).to.be.true();
      expect(mockRpcClient.getNetworkInfo.notCalled).to.be.true();
      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();

      expect(mockGithubProvider.notCalled).to.be.true();
      expect(mockMNOWatchProvider.notCalled).to.be.true();
      expect(mockInsightProvider().status.notCalled).to.be.true();
    });

    it('should set service error if couldnt get core data', async function it() {
      mockDockerCompose.isServiceRunning.resolves(true);
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.returns(Promise.reject());
      mockRpcClient.getNetworkInfo.returns({ result: { subversion: '' } });
      mockRpcClient.getBlockchainInfo.returns({
        result:
          {
            size_on_disk: 1337,
            verificationprogress: 1,
          },
      });

      mockGithubProvider.returns('v1337-dev');
      mockMNOWatchProvider.returns('OPEN');
      mockInsightProvider.returns({
        status: this.sinon.stub().returns({
          info: { blocks: 1337 },
        }),
      });

      const scope = await getCoreScope(config);

      const expectedScope = {
        network,
        p2pService,
        rpcService,
        version: null,
        chain: null,
        latestVersion: 'v1337-dev',
        dockerStatus: DockerStatusEnum.running,
        serviceStatus: ServiceStatusEnum.error,
        peersCount: null,
        p2pPortState: 'OPEN',
        blockHeight: null,
        remoteBlockHeight: 1337,
        headerHeight: null,
        difficulty: null,
        verificationProgress: null,
        sizeOnDisk: null,
        syncAsset: null,
      };

      expect(scope).to.be.deep.equal(expectedScope);
    });

    it('should omit providers data if error is thrown', async () => {
      mockDockerCompose.isServiceRunning.resolves(true);
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.returns({
        result:
          { AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED },
      });
      mockRpcClient.getNetworkInfo.returns({ result: { subversion: '/Dash Core:0.17.0.3/', connections: 1 } });
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          difficulty: 1,
          blocks: 2,
          headers: 3,
          chain: 'test',
          size_on_disk: 1337,
          verificationprogress: 1,
        },
      });

      mockGithubProvider.returns(Promise.reject());
      mockMNOWatchProvider.returns(PortStateEnum.ERROR);
      mockInsightProvider.returns({
        status: () => Promise.reject(),
      });

      const scope = await getCoreScope(config);

      const expectedScope = {
        network,
        p2pService: '127.0.0.1:8081',
        rpcService: '127.0.0.1:8080',
        version: '0.17.0.3',
        chain: 'test',
        latestVersion: null,
        dockerStatus: DockerStatusEnum.running,
        serviceStatus: ServiceStatusEnum.up,
        peersCount: 1,
        p2pPortState: PortStateEnum.ERROR,
        blockHeight: 2,
        remoteBlockHeight: null,
        headerHeight: 3,
        difficulty: 1,
        verificationProgress: 1,
        sizeOnDisk: 1337,
        syncAsset: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
      };

      expect(scope).to.deep.equal(expectedScope);
    });
  });
});
