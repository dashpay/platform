const MasternodeSyncAssetEnum = require('../../../../src/enums/masternodeSyncAsset');
const ServiceStatusEnum = require('../../../../src/enums/serviceStatus');
const DockerStatusEnum = require('../../../../src/enums/dockerStatus');
const getCoreScopeFactory = require('../../../../src/status/scopes/core');
const determineStatus = require('../../../../src/status/determineStatus');
const providers = require('../../../../src/status/providers');
const ServiceIsNotRunningError = require('../../../../src/docker/errors/ServiceIsNotRunningError');

describe('getCoreScopeFactory', () => {
  describe('#getCoreScope', () => {
    let mockRpcClient;
    let mockCreateRpcClient;
    let mockDockerCompose;
    let mockDetermineDockerStatus;
    let mockGithubProvider;
    let mockMNOWatchProvider;
    let mockInsightProvider;

    let config;
    let getCoreScope;

    beforeEach(async function it() {
      mockRpcClient = {
        mnsync: this.sinon.stub(),
        getBlockchainInfo: this.sinon.stub(),
        getNetworkInfo: this.sinon.stub(),
      };
      mockCreateRpcClient = () => mockRpcClient;
      mockDockerCompose = { isServiceRunning: this.sinon.stub() };
      mockDetermineDockerStatus = this.sinon.stub(determineStatus, 'docker');
      mockGithubProvider = this.sinon.stub(providers.github, 'release');
      mockMNOWatchProvider = this.sinon.stub(providers.mnowatch, 'checkPortStatus');
      mockInsightProvider = this.sinon.stub(providers, 'insight');

      config = { get: this.sinon.stub(), toEnvs: this.sinon.stub() };
      getCoreScope = getCoreScopeFactory(mockDockerCompose, mockCreateRpcClient);
    });

    it('should just work', async function it() {
      config.get.withArgs('network').returns('mainnet');
      config.get.withArgs('core.rpc.port').returns('80');
      config.get.withArgs('core.p2p.port').returns('8080');
      config.get.withArgs('externalIp').returns('localhost');

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

      expect(scope.network).to.be.equal('mainnet');
      expect(scope.p2pService).to.be.equal('localhost:8080');
      expect(scope.rpcService).to.be.equal('127.0.0.1:80');
      expect(scope.dockerStatus).to.be.equal(DockerStatusEnum.running);
      expect(scope.serviceStatus).to.be.equal(ServiceStatusEnum.up);
      expect(scope.chain).to.be.equal('test');
      expect(scope.difficulty).to.be.equal(1);
      expect(scope.blockHeight).to.be.equal(2);
      expect(scope.headerHeight).to.be.equal(3);
      expect(scope.verificationProgress).to.be.equal(1);
      expect(scope.peersCount).to.be.equal(1);
      expect(scope.version).to.be.equal('0.17.0.3');
      expect(scope.latestVersion).to.be.equal('v1337-dev');
      expect(scope.p2pPortState).to.be.equal('OPEN');
      expect(scope.remoteBlockHeight).to.be.equal(1337);
      expect(scope.sizeOnDisk).to.be.equal(1337);
    });

    it('should throw ServiceIsNotRunning error if core is not started', async () => {
      mockDockerCompose.isServiceRunning.resolves(false);

      try {
        await getCoreScope(config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e instanceof ServiceIsNotRunningError).to.be.true();
      }
    });

    it('should not make any requests if docker status is bad', async function it() {
      config.get.withArgs('network').returns('mainnet');
      config.get.withArgs('core.rpc.port').returns('80');
      config.get.withArgs('core.p2p.port').returns('8080');
      config.get.withArgs('externalIp').returns('localhost');

      mockDockerCompose.isServiceRunning.resolves(true);
      mockDetermineDockerStatus.returns(DockerStatusEnum.restarting);
      mockInsightProvider.returns({ status: this.sinon.stub() });

      const scope = await getCoreScope(config);

      expect(scope.network).to.be.equal('mainnet');
      expect(scope.p2pService).to.be.equal('localhost:8080');
      expect(scope.rpcService).to.be.equal('127.0.0.1:80');
      expect(scope.dockerStatus).to.be.equal(DockerStatusEnum.restarting);
      expect(scope.serviceStatus).to.be.equal(ServiceStatusEnum.error);

      expect(mockRpcClient.mnsync.notCalled).to.be.true();
      expect(mockRpcClient.getNetworkInfo.notCalled).to.be.true();
      expect(mockRpcClient.getBlockchainInfo.notCalled).to.be.true();

      expect(mockGithubProvider.notCalled).to.be.true();
      expect(mockMNOWatchProvider.notCalled).to.be.true();
      expect(mockInsightProvider().status.notCalled).to.be.true();
    });

    it('should omit providers data if error is thrown', async function it() {
      mockRpcClient.mnsync.returns({
        result: {
          AssetName: MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED,
        },
      });
      mockRpcClient.getNetworkInfo.returns({ result: { subversion: '' } });
      mockRpcClient.getBlockchainInfo.returns({
        result:
          { size_on_disk: 1337, verificationprogress: 1 },
      });
      mockDockerCompose.isServiceRunning.resolves(true);
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);

      mockGithubProvider.returns(Promise.reject());
      mockMNOWatchProvider.returns(Promise.reject());
      mockInsightProvider.returns({ status: this.sinon.stub().returns(Promise.reject()) });

      const scope = await getCoreScope(config);

      expect(scope.serviceStatus).to.be.equal(ServiceStatusEnum.up);
      expect(scope.verificationProgress).to.be.equal(1);

      expect(scope.latestVersion).to.be.equal(null);
      expect(scope.p2pPortState).to.be.equal(null);
      expect(scope.remoteBlockHeight).to.be.equal(null);
    });

    it('should throw if error during request to core', async () => {
      mockRpcClient.mnsync.throws(new Error());
      mockDockerCompose.isServiceRunning.resolves(true);

      try {
        await getCoreScope(config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e instanceof Error).to.be.true();
      }
    });
  });
});
