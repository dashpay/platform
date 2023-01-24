const fetch = require('node-fetch');

const { FetchError } = fetch;

const getPlatformScopeFactory = require('../../../../src/status/scopes/platform');
const determineStatus = require('../../../../src/status/determineStatus');
const DockerStatusEnum = require('../../../../src/enums/dockerStatus');
const providers = require('../../../../src/status/providers');
const ServiceStatusEnum = require('../../../../src/enums/serviceStatus');

describe('getPlatformScopeFactory', () => {
  describe('#getPlatformScope', () => {
    let mockRpcClient;
    let mockCreateRpcClient;
    let mockDetermineDockerStatus;
    let mockMNOWatchProvider;
    let mockFetch;
    let mockDockerCompose;
    let mockGetConnectionHost;

    let config;
    let getPlatformScope;

    beforeEach(async function it() {
      mockRpcClient = {
        mnsync: this.sinon.stub(),
        getBlockchainInfo: this.sinon.stub(),
        masternode: this.sinon.stub(),
      };
      mockDockerCompose = {
        execCommand: this.sinon.stub(),
        getContainerIp: this.sinon.stub(),
      };
      mockCreateRpcClient = () => mockRpcClient;
      mockDetermineDockerStatus = this.sinon.stub(determineStatus, 'docker');
      mockMNOWatchProvider = this.sinon.stub(providers.mnowatch, 'checkPortStatus');
      mockFetch = this.sinon.stub(fetch, 'Promise');
      mockGetConnectionHost = this.sinon.stub();

      config = { get: this.sinon.stub(), toEnvs: this.sinon.stub() };
      getPlatformScope = getPlatformScopeFactory(mockDockerCompose,
        mockCreateRpcClient, mockGetConnectionHost);
    });

    it('should just work', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.returns({ result: { IsSynced: true } });
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });

      const externalIp = '192.168.0.1';

      config.get.withArgs('platform.dapi.envoy.http.port').returns('8100');
      config.get.withArgs('platform.dapi.envoy.grpc.port').returns('8101');
      config.get.withArgs('platform.drive.tenderdash.p2p.port').returns('8200');
      config.get.withArgs('platform.drive.tenderdash.rpc.port').returns('8201');
      config.get.withArgs('externalIp').returns(externalIp);

      const mockStatus = {
        node_info: {
          version: '0',
          network: 'test',
        },
        sync_info: {
          catching_up: false,
          latestAppHash: '',
        },
      };
      const mockNetInfo = { n_peer: 3 };

      mockFetch
        .onFirstCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockStatus) }))
        .onSecondCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockNetInfo) }));
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const scope = await getPlatformScope(config);

      expect(scope.coreIsSynced).to.be.equal(true);

      expect(scope.httpService).to.be.equal(`${externalIp}:8100`);
      expect(scope.gRPCService).to.be.equal(`${externalIp}:8101`);
      expect(scope.p2pService).to.be.equal(`${externalIp}:8200`);
      expect(scope.rpcService).to.be.equal('127.0.0.1:8201');
      expect(scope.httpPortState).to.be.equal('OPEN');
      expect(scope.gRPCPortState).to.be.equal('OPEN');
      expect(scope.p2pPortState).to.be.equal('OPEN');
    });

    it('should throw if error during request to docker', async () => {
      mockDetermineDockerStatus.throws(new Error());

      try {
        await getPlatformScope(config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e instanceof Error).to.be.true();
      }
    });

    it('should throw if error during mnsync', async () => {
      mockRpcClient.mnsync.throws(new Error());

      try {
        await getPlatformScope(config);

        expect.fail('should throw error');
      } catch (e) {
        expect(e instanceof Error).to.be.true();
      }
    });

    it('should have error service status in case FetchError to tenderdash', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.returns({ result: { IsSynced: true } });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));
      mockFetch.returns(Promise.reject(new FetchError()));
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });

      const scope = await getPlatformScope(config);

      expect(scope.coreIsSynced).to.be.equal(true);
      expect(scope.httpPortState).to.be.equal(null);
      expect(scope.gRPCPortState).to.be.equal(null);
      expect(scope.p2pPortState).to.be.equal(null);
      expect(scope.tenderdash.dockerStatus).to.be.equal(DockerStatusEnum.running);
      expect(scope.tenderdash.serviceStatus).to.be.equal(ServiceStatusEnum.error);
      expect(scope.tenderdash.version).to.be.equal(null);
      expect(scope.tenderdash.catchingUp).to.be.equal(null);
      expect(scope.tenderdash.lastBlockHeight).to.be.equal(null);
      expect(scope.tenderdash.latestAppHash).to.be.equal(null);
    });
  });
});
