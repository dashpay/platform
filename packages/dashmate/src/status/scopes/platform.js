const fetch = require('node-fetch');
const determineStatus = require('../determineStatus');
const ServiceStatusEnum = require('../../enums/serviceStatus');
const providers = require('../providers');

/**
 * @returns {getPlatformScopeFactory}
 * @param dockerCompose {DockerCompose}
 * @param createRpcClient {createRpcClient}
 * @param getConnectionHost {getConnectionHost}
 */
function getPlatformScopeFactory(dockerCompose,
  createRpcClient, getConnectionHost) {
  /**
   * Get platform status scope
   *
   * @typedef {Promise} getPlatformScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getPlatformScope(config) {
    const hosts = {
      core: await getConnectionHost(config, 'core'),
      drive: await dockerCompose.getContainerIp(config.toEnvs(), 'drive_abci'),
      tenderdash: await getConnectionHost(config, 'drive_tenderdash'),
    };

    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
      host: hosts.core,
    });

    const httpPort = config.get('platform.dapi.envoy.http.port');
    const httpService = `${config.get('externalIp')}:${httpPort}`;
    const gRPCPort = config.get('platform.dapi.envoy.grpc.port');
    const gRPCService = `${config.get('externalIp')}:${gRPCPort}`;
    const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
    const p2pService = `${config.get('externalIp')}:${p2pPort}`;
    const rpcService = `127.0.0.1:${config.get('platform.drive.tenderdash.rpc.port')}`;

    const {
      result: {
        IsSynced: coreIsSynced,
      },
    } = await rpcClient.mnsync('status');

    const tenderdashDockerStatus = await determineStatus.docker(dockerCompose, config, 'drive_tenderdash');
    const driveDockerStatus = await determineStatus.docker(dockerCompose, config, 'drive_abci');

    const tenderdashServiceStatus = determineStatus.platform(tenderdashDockerStatus, coreIsSynced);
    let driveServiceStatus = determineStatus.platform(driveDockerStatus, coreIsSynced);

    const driveEchoResult = await dockerCompose.execCommand(config.toEnvs(),
      'drive_abci', 'yarn workspace @dashevo/drive echo');

    if (driveEchoResult.exitCode !== 0) {
      // eslint-disable-next-line no-console
      console.error(driveEchoResult.out);

      driveServiceStatus = ServiceStatusEnum.error;
    }

    const platform = {
      coreIsSynced,
      httpPort,
      httpService,
      p2pPort,
      p2pService,
      gRPCPort,
      gRPCService,
      rpcService,
      httpPortState: null,
      gRPCPortState: null,
      p2pPortState: null,
      tenderdash: {
        dockerStatus: tenderdashDockerStatus,
        serviceStatus: tenderdashServiceStatus,
        version: null,
        listening: null,
        catchingUp: null,
        lastBlockHash: null,
        lastBlockHeight: null,
        latestAppHash: null,
        peers: null,
        moniker: null,
        network: null,
      },
      drive: {
        dockerStatus: driveDockerStatus,
        serviceStatus: driveServiceStatus,
      },
    };

    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (tenderdashServiceStatus === ServiceStatusEnum.up) {
      try {
        const [tenderdashStatusResponse, tenderdashNetInfoResponse] = await Promise.all([
          fetch(`http://${hosts.tenderdash}:${config.get('platform.drive.tenderdash.rpc.port')}/status`),
          fetch(`http://${hosts.tenderdash}:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`),
        ]);

        const [httpPortState, gRPCPortState, p2pPortState] = await Promise.all([
          providers.mnowatch.checkPortStatus(httpPort),
          providers.mnowatch.checkPortStatus(gRPCPort),
          providers.mnowatch.checkPortStatus(p2pPort),
        ]);

        const [tenderdashStatus, tenderdashNetInfo] = await Promise.all([
          tenderdashStatusResponse.json(),
          tenderdashNetInfoResponse.json(),
        ]);

        platform.httpPortState = httpPortState;
        platform.gRPCPortState = gRPCPortState;
        platform.p2pPortState = p2pPortState;

        const { version, network, moniker } = tenderdashStatus.node_info;

        const catchingUp = tenderdashStatus.sync_info.catching_up;
        const lastBlockHeight = tenderdashStatus.sync_info.latest_block_height;
        const lastBlockHash = tenderdashStatus.sync_info.latest_block_hash;
        const latestAppHash = tenderdashStatus.sync_info.latest_app_hash;

        const platformPeers = parseInt(tenderdashNetInfo.n_peers, 10);
        const { listening } = tenderdashNetInfo;

        platform.tenderdash.version = version;
        platform.tenderdash.listening = listening;
        platform.tenderdash.lastBlockHeight = lastBlockHeight;
        platform.tenderdash.lastBlockHash = lastBlockHash;
        platform.tenderdash.catchingUp = catchingUp;
        platform.tenderdash.peers = platformPeers;
        platform.tenderdash.moniker = moniker;
        platform.tenderdash.network = network;
        platform.tenderdash.latestAppHash = latestAppHash;
      } catch (e) {
        if (e.name === 'FetchError') {
          platform.tenderdash.serviceStatus = ServiceStatusEnum.error;

          // eslint-disable-next-line no-console
          console.error(e);
        } else {
          throw e;
        }
      }
    }

    return platform;
  }

  return getPlatformScope;
}

module.exports = getPlatformScopeFactory;
