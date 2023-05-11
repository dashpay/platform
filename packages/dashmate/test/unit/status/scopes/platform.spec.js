const fetch = require('node-fetch');

const {FetchError} = fetch;

const getPlatformScopeFactory = require('../../../../src/status/scopes/platform');
const determineStatus = require('../../../../src/status/determineStatus');
const DockerStatusEnum = require('../../../../src/status/enums/dockerStatus');
const providers = require('../../../../src/status/providers');
const ServiceStatusEnum = require('../../../../src/status/enums/serviceStatus');

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
    let httpServiceUrl
    let p2pPort
    let p2pServiceUrl
    let rpcPort
    let rpcServiceUrl

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

      httpPort = config.get('platform.dapi.envoy.http.port');
      httpServiceUrl = `${config.get('externalIp')}:${httpPort}`;
      p2pPort = config.get('platform.drive.tenderdash.p2p.port');
      p2pServiceUrl = `${config.get('externalIp')}:${p2pPort}`;
      rpcPort = config.get('platform.drive.tenderdash.rpc.port');
      rpcServiceUrl = `127.0.0.1:${rpcPort}`;

      getPlatformScope = getPlatformScopeFactory(mockDockerCompose,
        mockCreateRpcClient, mockGetConnectionHost);
    });

    it('should just work', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.returns({result: {IsSynced: true}});
      mockDockerCompose.execCommand.returns({exitCode: 0, out: ''});

      const externalIp = '192.168.0.1';

      config.get.withArgs('platform.dapi.envoy.http.port').returns('8100');
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
      const mockNetInfo = {n_peer: 3};

      mockFetch
        .onFirstCall()
        .returns(Promise.resolve({json: () => Promise.resolve(mockStatus)}))
        .onSecondCall()
        .returns(Promise.resolve({json: () => Promise.resolve(mockNetInfo)}));
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const scope = await getPlatformScope(config);

      expect(scope.coreIsSynced).to.be.equal(true);

      expect(scope.httpService).to.be.equal(`${externalIp}:8100`);
      expect(scope.p2pService).to.be.equal(`${externalIp}:8200`);
      expect(scope.rpcService).to.be.equal('127.0.0.1:8201');
      expect(scope.httpPortState).to.be.equal('OPEN');
      expect(scope.p2pPortState).to.be.equal('OPEN');
    });

    it('should return scope if error during request to docker', async () => {
      mockDetermineDockerStatus.throws(new Error());

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
