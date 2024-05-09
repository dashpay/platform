import providers from '../../../../src/status/providers.js';
import determineStatus from '../../../../src/status/determineStatus.js';
import getConfigMock from '../../../../src/test/mock/getConfigMock.js';
import getPlatformScopeFactory from '../../../../src/status/scopes/platform.js';
import { DockerStatusEnum } from '../../../../src/status/enums/dockerStatus.js';
import { PortStateEnum } from '../../../../src/status/enums/portState.js';
import { ServiceStatusEnum } from '../../../../src/status/enums/serviceStatus.js';

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
    let httpPort;
    let httpService;
    let p2pPort;
    let p2pService;
    let rpcPort;
    let rpcService;

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
        isNodeRunning: this.sinon.stub(),
        isServiceRunning: this.sinon.stub(),
      };
      mockCreateRpcClient = () => mockRpcClient;
      mockDetermineDockerStatus = this.sinon.stub(determineStatus, 'docker');
      mockMNOWatchProvider = this.sinon.stub(providers.mnowatch, 'checkPortStatus');
      // eslint-disable-next-line
      mockFetch = this.sinon.stub(globalThis, 'fetch');
      mockGetConnectionHost = this.sinon.stub();

      config = getConfigMock(this.sinon);

      httpPort = config.get('platform.dapi.envoy.http.port');
      httpService = `${config.get('externalIp')}:${httpPort}`;
      p2pPort = config.get('platform.drive.tenderdash.p2p.port');
      p2pService = `${config.get('externalIp')}:${p2pPort}`;
      rpcPort = config.get('platform.drive.tenderdash.rpc.port');
      rpcService = `127.0.0.1:${rpcPort}`;
      getPlatformScope = getPlatformScopeFactory(
        mockDockerCompose,
        mockCreateRpcClient,
        mockGetConnectionHost,
      );
    });

    it('should just work', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: true } });
      mockDockerCompose.isServiceRunning.returns(true);
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });
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
      const mockNetInfo = { n_peers: 6, listening: true };

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
          serviceStatus: ServiceStatusEnum.up,
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
        .returns(Promise.resolve({ json: () => Promise.resolve(mockStatus) }))
        .onSecondCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockNetInfo) }));
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return platform syncing when it is catching up', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: true } });
      mockDockerCompose.isServiceRunning.returns(true);
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const mockStatus = {
        node_info: {
          version: '0',
          network: 'test',
          moniker: 'test',
        },
        sync_info: {
          catching_up: true,
          latest_app_hash: 'DEADBEEF',
          latest_block_height: 1337,
          latest_block_hash: 'DEADBEEF',
          latest_block_time: 1337,
        },
      };
      const mockNetInfo = { n_peers: 6, listening: true };

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
          serviceStatus: ServiceStatusEnum.syncing,
          version: '0',
          listening: true,
          catchingUp: true,
          latestBlockHash: 'DEADBEEF',
          latestBlockHeight: 1337,
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
        .returns(Promise.resolve({ json: () => Promise.resolve(mockStatus) }))
        .onSecondCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockNetInfo) }));
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return empty scope if error during request to core', async () => {
      mockRpcClient.mnsync.withArgs('status').throws(new Error());
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });
      mockDockerCompose.isServiceRunning.returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash')
        .returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci')
        .returns(DockerStatusEnum.running);

      const expectedScope = {
        coreIsSynced: null,
        httpPort,
        httpService,
        p2pPort,
        p2pService,
        rpcService,
        httpPortState: null,
        p2pPortState: null,
        tenderdash: {
          httpPortState: null,
          p2pPortState: null,
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.wait_for_core,
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
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.wait_for_core,
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return empty scope if core is not synced', async () => {
      mockDockerCompose.isServiceRunning
        .withArgs(config, 'drive_tenderdash')
        .returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash').returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci').returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: false } });
      mockDockerCompose.execCommand.returns({ exitCode: 1, out: '' });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const expectedScope = {
        coreIsSynced: false,
        httpPort,
        httpService,
        p2pPort,
        p2pService,
        rpcService,
        httpPortState: null,
        p2pPortState: null,
        tenderdash: {
          httpPortState: null,
          p2pPortState: null,
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.wait_for_core,
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
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.wait_for_core,
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return drive info if tenderdash is failed', async () => {
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: true } });
      mockDockerCompose.isServiceRunning
        .withArgs(config, 'drive_tenderdash')
        .returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash')
        .returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci')
        .returns(DockerStatusEnum.running);
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

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
          serviceStatus: ServiceStatusEnum.error,
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
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.up,
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should still return scope with tenderdash if drive is failed', async () => {
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: true } });
      mockDockerCompose.isServiceRunning
        .withArgs(config, 'drive_tenderdash')
        .returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash')
        .returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci')
        .throws();
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });
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
      const mockNetInfo = { n_peers: 6, listening: true };

      mockFetch
        .onFirstCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockStatus) }))
        .onSecondCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockNetInfo) }));

      const expectedScope = {
        coreIsSynced: true,
        httpPort,
        httpService,
        p2pPort,
        p2pService,
        rpcService,
        httpPortState: 'OPEN',
        p2pPortState: 'OPEN',
        tenderdash: {
          httpPortState: 'OPEN',
          p2pPortState: 'OPEN',
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.up,
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
          dockerStatus: null,
          serviceStatus: null,
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should have error service status in case FetchError to tenderdash', async () => {
      mockRpcClient.mnsync.returns({ result: { IsSynced: true } });
      mockDockerCompose.isServiceRunning
        .withArgs(config, 'drive_tenderdash')
        .returns(true);
      mockDockerCompose.execCommand.returns({ exitCode: 0, out: '' });
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));
      mockFetch.returns(Promise.reject(new Error('FetchError')));

      const expectedScope = {
        coreIsSynced: true,
        httpPort,
        httpService,
        p2pPort,
        p2pService,
        rpcService,
        httpPortState: 'OPEN',
        p2pPortState: 'OPEN',
        tenderdash: {
          httpPortState: 'OPEN',
          p2pPortState: 'OPEN',
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.error,
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
          dockerStatus: DockerStatusEnum.running,
          serviceStatus: ServiceStatusEnum.up,
        },
      };

      const scope = await getPlatformScope(config);
      expect(scope).to.deep.equal(expectedScope);
    });
  });
});
