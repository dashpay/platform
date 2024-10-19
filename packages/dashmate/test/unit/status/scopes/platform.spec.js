import ContainerIsNotPresentError
  from '../../../../src/docker/errors/ContainerIsNotPresentError.js';
import DockerComposeError from '../../../../src/docker/errors/DockerComposeError.js';
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

      httpPort = config.get('platform.gateway.listeners.dapiAndDrive.port');
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
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          softforks: {
            mn_rr: { active: true, height: 1337 },
          },
        },
      });
      mockDockerCompose.isServiceRunning.returns(true);
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci status').resolves({ exitCode: 0, out: '' });
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci version').resolves({ exitCode: 0, out: '1.4.1' });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const mockStatus = {
        node_info: {
          protocol_version: {
            p2p: '10',
            block: '14',
            app: '3',
          },
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

      const mockAbciInfo = {
        response: {
          version: '1.4.1',
          app_version: 4,
          last_block_height: 90,
          last_block_app_hash: 's0CySQxgRg96DrnJ7HCsql+k/Sk4JiT3y0psCaUI3TI=',
        },
      };

      const expectedScope = {
        platformActivation: 'Activated (at height 1337)',
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
          protocolVersion: 3,
          desiredProtocolVersion: 4,
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
          version: '1.4.1',
        },
      };

      mockFetch
        .onFirstCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockStatus) }))
        .onSecondCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockNetInfo) }))
        .onThirdCall()
        .resolves({ json: () => Promise.resolve(mockAbciInfo) });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return platform syncing when it is catching up', async () => {
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: true } });
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          softforks: {
            mn_rr: { active: true, height: 1337 },
          },
        },
      });
      mockDockerCompose.isServiceRunning.returns(true);
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci version').resolves({ exitCode: 0, out: '1.4.1' });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const mockStatus = {
        node_info: {
          protocol_version: {
            p2p: '10',
            block: '14',
            app: '3',
          },
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

      const mockAbciInfo = {
        response: {
          version: '1.4.1',
          app_version: 4,
          last_block_height: 90,
          last_block_app_hash: 's0CySQxgRg96DrnJ7HCsql+k/Sk4JiT3y0psCaUI3TI=',
        },
      };

      const expectedScope = {
        platformActivation: 'Activated (at height 1337)',
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
          protocolVersion: 3,
          desiredProtocolVersion: 4,
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
          version: '1.4.1',
        },
      };

      mockFetch
        .onFirstCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockStatus) }))
        .onSecondCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockNetInfo) }))
        .onThirdCall()
        .resolves({ json: () => Promise.resolve(mockAbciInfo) });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return empty scope if error during request to core', async () => {
      mockRpcClient.mnsync.withArgs('status').throws(new Error());
      mockDockerCompose.isServiceRunning.returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash')
        .returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci')
        .returns(DockerStatusEnum.running);

      const expectedScope = {
        platformActivation: null,
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
          dockerStatus: null,
          serviceStatus: null,
          version: null,
          protocolVersion: null,
          desiredProtocolVersion: null,
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
          version: null,
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return empty scope if core is not synced', async () => {
      mockDockerCompose.isServiceRunning.withArgs(config, 'drive_tenderdash').returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash').returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci').returns(DockerStatusEnum.running);
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: false } });
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          softforks: {
            mn_rr: { active: true, height: 1337 },
          },
        },
      });
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci version').resolves({ exitCode: 0, out: '1.4.1' });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const expectedScope = {
        platformActivation: 'Activated (at height 1337)',
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
          protocolVersion: null,
          desiredProtocolVersion: null,
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
          version: '1.4.1',
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should return drive info if tenderdash is failed', async () => {
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: true } });
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          softforks: {
            mn_rr: { active: true, height: 1337 },
          },
        },
      });
      mockDockerCompose.isServiceRunning
        .withArgs(config, 'drive_tenderdash')
        .returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash')
        .returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci')
        .returns(DockerStatusEnum.running);
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci version').resolves({ exitCode: 0, out: '1.4.1' });
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));

      const expectedScope = {
        platformActivation: 'Activated (at height 1337)',
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
          protocolVersion: null,
          desiredProtocolVersion: null,
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
          version: '1.4.1',
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should still return scope with tenderdash if drive is failed', async () => {
      mockRpcClient.mnsync.withArgs('status').returns({ result: { IsSynced: true } });
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          softforks: {
            mn_rr: { active: true, height: 1337 },
          },
        },
      });
      mockDockerCompose.isServiceRunning
        .withArgs(config, 'drive_tenderdash')
        .returns(true);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_tenderdash')
        .returns(DockerStatusEnum.running);
      mockDetermineDockerStatus.withArgs(mockDockerCompose, config, 'drive_abci')
        .throws(new ContainerIsNotPresentError('drive_abci'));
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));
      const error = new DockerComposeError({
        exitCode: 1,
      });
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci version').rejects(error);

      const mockStatus = {
        node_info: {
          protocol_version: {
            p2p: '10',
            block: '14',
            app: '3',
          },
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

      const mockAbciInfo = {
        response: {
          version: '1.4.1',
          app_version: 4,
          last_block_height: 90,
          last_block_app_hash: 's0CySQxgRg96DrnJ7HCsql+k/Sk4JiT3y0psCaUI3TI=',
        },
      };

      mockFetch
        .onFirstCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockStatus) }))
        .onSecondCall()
        .returns(Promise.resolve({ json: () => Promise.resolve(mockNetInfo) }))
        .onThirdCall()
        .resolves({ json: () => Promise.resolve(mockAbciInfo) });

      const expectedScope = {
        platformActivation: 'Activated (at height 1337)',
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
          protocolVersion: 3,
          desiredProtocolVersion: 4,
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
          dockerStatus: DockerStatusEnum.not_started,
          serviceStatus: ServiceStatusEnum.stopped,
          version: null,
        },
      };

      const scope = await getPlatformScope(config);

      expect(scope).to.deep.equal(expectedScope);
    });

    it('should have error service status in case FetchError to tenderdash', async () => {
      mockRpcClient.mnsync.returns({ result: { IsSynced: true } });
      mockRpcClient.getBlockchainInfo.returns({
        result: {
          softforks: {
            mn_rr: { active: true, height: 1337 },
          },
        },
      });
      mockDockerCompose.isServiceRunning
        .withArgs(config, 'drive_tenderdash')
        .returns(true);
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci status').resolves({ exitCode: 0, out: '' });
      mockDockerCompose.execCommand.withArgs(config, 'drive_abci', 'drive-abci version').resolves({ exitCode: 0, out: '1.4.1' });
      mockDetermineDockerStatus.returns(DockerStatusEnum.running);
      mockMNOWatchProvider.returns(Promise.resolve('OPEN'));
      mockFetch.returns(Promise.reject(new Error('FetchError')));

      const expectedScope = {
        platformActivation: 'Activated (at height 1337)',
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
          protocolVersion: null,
          desiredProtocolVersion: null,
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
          version: '1.4.1',
        },
      };

      const scope = await getPlatformScope(config);
      expect(scope).to.deep.equal(expectedScope);
    });
  });
});
