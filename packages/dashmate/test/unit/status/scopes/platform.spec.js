const fetch = require('node-fetch');

const {FetchError} = fetch;

const getPlatformScopeFactory = require('../../../../src/status/scopes/platform');
const determineStatus = require('../../../../src/status/determineStatus');
const DockerStatusEnum = require('../../../../src/status/enums/dockerStatus');
const providers = require('../../../../src/status/providers');
const ServiceStatusEnum = require('../../../../src/status/enums/serviceStatus');
const MasternodeStateEnum = require("../../../../src/status/enums/masternodeState");
const PortStateEnum = require("../../../../src/status/enums/portState");

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
    let httpPort
    let httpService
    let p2pPort
    let p2pService
    let rpcPort
    let rpcService

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

      config = {
        get: this.sinon.stub(),
        toEnvs: this.sinon.stub(),
      };

      config.get.withArgs('platform.dapi.envoy.http.port').returns('8100');
      config.get.withArgs('externalIp').returns('127.0.0.1');
      config.get.withArgs('platform.drive.tenderdash.p2p.port').returns('8101');
      config.get.withArgs('platform.dapi.envoy.http.port').returns('8102');
      config.get.withArgs('platform.drive.tenderdash.rpc.port').returns('8103');

      httpPort = config.get('platform.dapi.envoy.http.port');
      httpService = `${config.get('externalIp')}:${httpPort}`;
      p2pPort = config.get('platform.drive.tenderdash.p2p.port');
      p2pService = `${config.get('externalIp')}:${p2pPort}`;
      rpcPort = config.get('platform.drive.tenderdash.rpc.port');
      rpcService = `127.0.0.1:${rpcPort}`;
      getPlatformScope = getPlatformScopeFactory(mockDockerCompose,
        mockCreateRpcClient, mockGetConnectionHost);
    });

    it.only('should just work', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.withArgs('status').returns({result: {IsSynced: true}});
      mockDockerCompose.execCommand.returns({exitCode: 0, out: ''});
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const mockStatus = {
        node_info: {
          version: '0',
          network: 'test',
          moniker: 'test',
        },
        sync_info: {
          catching_up: false,
          latest_app_hash: 'DEADBEEF',
          latest_block_height: 1,
          latest_block_hash: 'DEADBEEF',
          latest_block_time: 1337,
        },
      };
      const mockNetInfo = {n_peers: 6, listening: true};

      const expectedScope = {
        coreIsSynced: true,
        httpPort,
        httpService,
        p2pPort,
        p2pService,
        rpcService,
        httpPortState: PortStateEnum.OPEN,
        p2pPortState: PortStateEnum.OPEN,
        tenderdash: {
          httpPortState: PortStateEnum.OPEN,
          p2pPortState: PortStateEnum.OPEN,
          dockerStatus: DockerStatusEnum.running,
          serviceStatus:  ServiceStatusEnum.up,
          version: '0',
          listening: true,
          catchingUp: false,
          latestBlockHash: 'DEADBEEF',
          latestBlockHeight: 1,
          latestBlockTime: 1337,
          latestAppHash: 'DEADBEEF',
          peers: 6,
          moniker: 'test',
          network: 'test',
        },
        drive: {
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.up,
        },
      };

      mockFetch
        .onFirstCall()
        .returns(Promise.resolve({json: () => Promise.resolve(mockStatus)}))
        .onSecondCall()
        .returns(Promise.resolve({json: () => Promise.resolve(mockNetInfo)}));
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return scope if error during request to docker', async () => {
      mockDetermineDockerStatus.throws(new Error());

      const expectedScope = {
        dapi: {
          httpPort,
          httpServiceUrl,
        },
        tenderdash: {
          p2pPort,
          p2pServiceUrl,
          rpcPort,
          rpcServiceUrl,
          httpPortState: null,
          p2pPortState: null,
          dockerStatus: null,
          serviceStatus: null,
          version: null,
          listening: null,
          catchingUp: null,
          latestBlockHash: null,
          latestBlockHeight: null,
          latestBlockTime: null,
          latestAppHash: null,
          peers: null,
          moniker: null,
          network: null,
        },
        drive: {
          dockerStatus: null,
          serviceStatus: null,
        },
      };

      const scope = await getPlatformScope(config);
      expect(scope).to.be.deepEqual(expectedScope);
    });

    it('should return scope if error during mnsync', async () => {
      mockRpcClient.masternode.throws(new Error());

      const httpPort = config.get('platform.dapi.envoy.http.port');
      const httpServiceUrl = `${config.get('externalIp')}:${httpPort}`;
      const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
      const p2pServiceUrl = `${config.get('externalIp')}:${p2pPort}`;
      const rpcPort = config.get('platform.drive.tenderdash.rpc.port');
      const rpcServiceUrl = `127.0.0.1:${rpcPort}`;

      const expectedScope = {
        dapi: {
          httpPort,
          httpServiceUrl,
        },
        tenderdash: {
          p2pPort,
          p2pServiceUrl,
          rpcPort,
          rpcServiceUrl,
          httpPortState: null,
          p2pPortState: null,
          dockerStatus: null,
          serviceStatus: null,
          version: null,
          listening: null,
          catchingUp: null,
          latestBlockHash: null,
          latestBlockHeight: null,
          latestBlockTime: null,
          latestAppHash: null,
          peers: null,
          moniker: null,
          network: null,
        },
        drive: {
          dockerStatus: null,
          serviceStatus: null,
        },
      };

      const scope = await getPlatformScope(config);
      expect(scope).to.be.deepEqual(expectedScope);
    });

    it('should return drive info even if tenderdash failed', async () => {
      mockRpcClient.masternode.throws(new Error());

      const httpPort = config.get('platform.dapi.envoy.http.port');
      const httpServiceUrl = `${config.get('externalIp')}:${httpPort}`;
      const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
      const p2pServiceUrl = `${config.get('externalIp')}:${p2pPort}`;
      const rpcPort = config.get('platform.drive.tenderdash.rpc.port');
      const rpcServiceUrl = `127.0.0.1:${rpcPort}`;

      const expectedScope = {
        dapi: {
          httpPort,
          httpServiceUrl,
        },
        tenderdash: {
          p2pPort,
          p2pServiceUrl,
          rpcPort,
          rpcServiceUrl,
          httpPortState: null,
          p2pPortState: null,
          dockerStatus: null,
          serviceStatus: null,
          version: null,
          listening: null,
          catchingUp: null,
          latestBlockHash: null,
          latestBlockHeight: null,
          latestBlockTime: null,
          latestAppHash: null,
          peers: null,
          moniker: null,
          network: null,
        },
        drive: {
          dockerStatus: 'running',
          serviceStatus: 'up',
        },
      };

      const scope = await getPlatformScope(config);
      expect(scope).to.be.deepEqual(expectedScope);
    });

    it('should return tenderdash info even if drive failed', async () => {
      mockRpcClient.masternode.throws(new Error());

      const httpPort = config.get('platform.dapi.envoy.http.port');
      const httpServiceUrl = `${config.get('externalIp')}:${httpPort}`;
      const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
      const p2pServiceUrl = `${config.get('externalIp')}:${p2pPort}`;
      const rpcPort = config.get('platform.drive.tenderdash.rpc.port');
      const rpcServiceUrl = `127.0.0.1:${rpcPort}`;

      const expectedScope = {
        dapi: {
          httpPort,
          httpServiceUrl,
        },
        tenderdash: {
          p2pPort,
          p2pServiceUrl,
          rpcPort,
          rpcServiceUrl,
          httpPortState: null,
          p2pPortState: null,
          dockerStatus: null,
          serviceStatus: null,
          version: null,
          listening: null,
          catchingUp: null,
          latestBlockHash: null,
          latestBlockHeight: null,
          latestBlockTime: null,
          latestAppHash: null,
          peers: null,
          moniker: null,
          network: null,
        },
        drive: {
          dockerStatus: 'running',
          serviceStatus: 'up',
        },
      };

      const scope = await getPlatformScope(config);
      expect(scope).to.be.deepEqual(expectedScope);
    });

    it('should have error service status in case FetchError to tenderdash', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.returns({result: {IsSynced: true}});
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));
      mockFetch.returns(Promise.reject(new FetchError('test')));
      mockDockerCompose.execCommand.returns({exitCode: 0, out: ''});

      const scope = await getPlatformScope(config);

      expect(scope.coreIsSynced).to.be.equal(true);
      expect(scope.httpPortState).to.be.equal(null);
      expect(scope.p2pPortState).to.be.equal(null);
      expect(scope.tenderdash.dockerStatus).to.be.equal(DockerStatusEnum.running);
      expect(scope.tenderdash.serviceStatus).to.be.equal(ServiceStatusEnum.error);
      expect(scope.tenderdash.version).to.be.equal(null);
      expect(scope.tenderdash.catchingUp).to.be.equal(null);
      expect(scope.tenderdash.latestBlockHeight).to.be.equal(null);
      expect(scope.tenderdash.latestBlockTime).to.be.equal(null);
      expect(scope.tenderdash.latestAppHash).to.be.equal(null);
    });
  });
});
