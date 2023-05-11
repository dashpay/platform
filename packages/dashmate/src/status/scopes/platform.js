const fetch = require('node-fetch');
const determineStatus = require('../determineStatus');
const ServiceStatusEnum = require('../../enums/serviceStatus');
const MasternodeStateEnum = require('../../enums/masternodeState');
const providers = require('../providers');

/**
 * @returns {getPlatformScopeFactory}
 * @param dockerCompose {DockerCompose}
 * @param createRpcClient {createRpcClient}
 * @param getConnectionHost {getConnectionHost}
 */
function getPlatformScopeFactory(dockerCompose,
                                 createRpcClient, getConnectionHost) {
  async function getMasternodeState() {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
      host: await getConnectionHost(config, 'core'),
    });

    const {
      result: {
        state,
      },
    } = await rpcClient.masternode('status');


    return state
  }

  async function handleTenderdash(scope, masternodeReady) {
    scope.tenderdash.docker.status = await determineStatus
      .docker(dockerCompose, config, 'drive_tenderdash');
    scope.tenderdash.service.status = determineStatus
      .platform(scope.tenderdash.docker.status, masternodeReady);

    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (scope.tenderdash.service.status === ServiceStatusEnum.up) {
      const [httpPortState, p2pPortState] = await Promise.all([
        providers.mnowatch.checkPortStatus(config.get('platform.dapi.envoy.http.port')),
        providers.mnowatch.checkPortStatus(config.get('platform.drive.tenderdash.p2p.port')),
      ]);

      scope.httpPortState = httpPortState;
      scope.p2pPortState = p2pPortState;

      try {
        const tenderdashHost = await getConnectionHost(config, 'drive_tenderdash')

        const [tenderdashStatusResponse, tenderdashNetInfoResponse] = await Promise.all([
          fetch(`http://${tenderdashHost}:${config.get('platform.drive.tenderdash.rpc.port')}/status`),
          fetch(`http://${tenderdashHost}:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`),
        ]);

        const [tenderdashStatus, tenderdashNetInfo] = await Promise.all([
          tenderdashStatusResponse.json(),
          tenderdashNetInfoResponse.json(),
        ]);

        const {version, network, moniker} = tenderdashStatus.node_info;

        const catchingUp = tenderdashStatus.sync_info.catching_up;
        const lastBlockHeight = tenderdashStatus.sync_info.latest_block_height;
        const lastBlockHash = tenderdashStatus.sync_info.latest_block_hash;
        const latestAppHash = tenderdashStatus.sync_info.latest_app_hash;

        const platformPeers = parseInt(tenderdashNetInfo.n_peers, 10);
        const {listening} = tenderdashNetInfo;

        scope.tenderdash.version = version;
        scope.tenderdash.listening = listening;
        scope.tenderdash.lastBlockHeight = lastBlockHeight;
        scope.tenderdash.lastBlockHash = lastBlockHash;
        scope.tenderdash.catchingUp = catchingUp;
        scope.tenderdash.peers = platformPeers;
        scope.tenderdash.moniker = moniker;
        scope.tenderdash.network = network;
        scope.tenderdash.latestAppHash = latestAppHash;
      } catch (e) {
        if (e.name === 'FetchError') {
          scope.tenderdash.serviceStatus = ServiceStatusEnum.error;
        } else {
          throw e;
        }
      }
    }
  }

  const handleDrive = async (scope, config, dockerCompose, masternodeReady) => {
    scope.drive.dockerStatus = await determineStatus.docker(dockerCompose, config, 'drive_abci');
    scope.drive.serviceStatus = determineStatus.platform(scope.drive.dockerStatus, masternodeReady);

    const driveEchoResult = await dockerCompose.execCommand(config.toEnvs(),
      'drive_abci', 'yarn workspace @dashevo/drive echo');

    if (driveEchoResult.exitCode !== 0) {
      scope.drive.serviceStatus = ServiceStatusEnum.error;
    }
  }

  /**
   * Get platform status scope
   *
   * @typedef {Promise} getPlatformScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getPlatformScope(config) {
    const httpPort = config.get('platform.dapi.envoy.http.port');
    const httpServiceUrl = `${config.get('externalIp')}:${httpPort}`;
    const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
    const p2pServiceUrl = `${config.get('externalIp')}:${p2pPort}`;
    const rpcPort = config.get('platform.drive.tenderdash.rpc.port');
    const rpcServiceUrl = `127.0.0.1:${rpcPort}`;

    const scope = {
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

    const state = await getMasternodeState(scope, config, createRpcClient, getConnectionHost);
    const masternodeReady = state === MasternodeStateEnum.READY;

    if (!masternodeReady) {
      console.error(`Platform status is not available until masternode state is 'READY'`)

      return scope
    }

    // handle simultaneously and mutate `scope`
    const result = await Promise.allSettled([
      handleTenderdash(scope, masternodeReady),
      handleDrive(scope, masternodeReady),
    ]);

    for (const error of result.filter(e => e.status === 'rejected')) {
      // eslint-disable-next-line no-console
      console.error(error.reason)
    }

    return scope
  }

  return getPlatformScope;
}

module.exports = getPlatformScopeFactory;
