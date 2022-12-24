const fetch = require('node-fetch');
const determineStatus = require('../determineStatus');
const ServiceStatusEnum = require('../../enums/serviceStatus');
const DockerStatusEnum = require('../../enums/dockerStatus');
const providers = require('../providers');

/**
 * @returns {getPlatformScopeFactory}
 * @param dockerCompose {DockerCompose}
 * @param createRpcClient {createRpcClient}
 */
function getPlatformScopeFactory(dockerCompose, createRpcClient) {
  /**
   * Get platform status scope
   *
   * @typedef {Promise} getPlatformScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getPlatformScope(config) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
    });

    const httpPort = config.get('platform.dapi.envoy.http.port');
    const httpService = `${config.get('externalIp')}:${httpPort}`;
    const gRPCPort = config.get('platform.dapi.envoy.grpc.port');
    const gRPCService = `${config.get('externalIp')}:${gRPCPort}`;
    const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
    const p2pService = `${config.get('externalIp')}:${p2pPort}`;
    const rpcService = `127.0.0.1:${config.get('platform.drive.tenderdash.rpc.port')}`;

    const dockerStatus = await determineStatus.docker(dockerCompose, config, 'drive_tenderdash');

    if (dockerStatus !== DockerStatusEnum.running) {
      throw new Error('drive_tenderdash container is not running');
    }

    const {
      result: {
        IsSynced: coreIsSynced,
      },
    } = await rpcClient.mnsync('status');

    const serviceStatus = determineStatus.platform(dockerStatus, coreIsSynced);

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
        dockerStatus,
        serviceStatus,
        version: null,
        catchingUp: null,
        lastBlockHeight: null,
        latestAppHash: null,
        peers: null,
        network: null,
      },
    };

    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (serviceStatus === ServiceStatusEnum.up) {
      try {
        const [tenderdashStatusResponse, tenderdashNetInfoResponse] = await Promise.all([
          fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/status`),
          fetch(`http://localhost:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`),
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

        const { version, network } = tenderdashStatus.node_info;

        const catchingUp = tenderdashStatus.sync_info.catching_up;
        const lastBlockHeight = tenderdashStatus.sync_info.latest_block_height;
        const latestAppHash = tenderdashStatus.sync_info.latest_app_hash;

        const platformPeers = tenderdashNetInfo.n_peers;

        platform.tenderdash.version = version;
        platform.tenderdash.lastBlockHeight = lastBlockHeight;
        platform.tenderdash.catchingUp = catchingUp;
        platform.tenderdash.peers = platformPeers;
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
