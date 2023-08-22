const determineStatus = require('../determineStatus');
const DockerStatusEnum = require('../enums/dockerStatus');
const ServiceStatusEnum = require('../enums/serviceStatus');
const providers = require('../providers');
const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');
const generateEnvs = require('../../util/generateEnvs');

/**
 * @returns {getPlatformScopeFactory}
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @param {ConfigFile} configFile
 */
function getPlatformScopeFactory(dockerCompose,
  createRpcClient, getConnectionHost, configFile) {
  async function getMNSync(config) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
      host: await getConnectionHost(config, 'core'),
    });

    const {
      result: {
        IsSynced: isSynced,
      },
    } = await rpcClient.mnsync('status');

    return isSynced;
  }

  async function getTenderdashInfo(config, isCoreSynced) {
    const info = {
      p2pPortState: null,
      httpPortState: null,
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
    };
    try {
      if (!(await dockerCompose.isServiceRunning(generateEnvs(configFile, config), 'drive_tenderdash'))) {
        info.dockerStatus = DockerStatusEnum.not_started;
        info.serviceStatus = ServiceStatusEnum.stopped;

        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.error('Platform (tenderdash) is not running');
        }

        return info;
      }

      const dockerStatus = await determineStatus.docker(dockerCompose, configFile, config, 'drive_tenderdash');
      const serviceStatus = determineStatus.platform(dockerStatus, isCoreSynced);

      info.dockerStatus = dockerStatus;
      info.serviceStatus = serviceStatus;
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error('Could not query docker for container status', e);

      return info;
    }

    // Collecting platform data fails if Tenderdash is waiting for core to sync
    if (info.serviceStatus === ServiceStatusEnum.up) {
      const portStatusResult = await Promise.allSettled([
        providers.mnowatch.checkPortStatus(config.get('platform.dapi.envoy.http.port')),
        providers.mnowatch.checkPortStatus(config.get('platform.drive.tenderdash.p2p.port')),
      ]);
      const [httpPortState, p2pPortState] = portStatusResult.map((result) => (result.status === 'fulfilled' ? result.value : null));

      info.httpPortState = httpPortState;
      info.p2pPortState = p2pPortState;

      try {
        const tenderdashHost = await getConnectionHost(config, 'drive_tenderdash');

        const [tenderdashStatusResponse, tenderdashNetInfoResponse] = await Promise.all([
          fetch(`http://${tenderdashHost}:${config.get('platform.drive.tenderdash.rpc.port')}/status`),
          fetch(`http://${tenderdashHost}:${config.get('platform.drive.tenderdash.rpc.port')}/net_info`),
        ]);

        const [tenderdashStatus, tenderdashNetInfo] = await Promise.all([
          tenderdashStatusResponse.json(),
          tenderdashNetInfoResponse.json(),
        ]);

        const { version, network, moniker } = tenderdashStatus.node_info;

        const catchingUp = tenderdashStatus.sync_info.catching_up;
        const latestBlockHeight = tenderdashStatus.sync_info.latest_block_height;
        const latestBlockTime = tenderdashStatus.sync_info.latest_block_time;
        const latestBlockHash = tenderdashStatus.sync_info.latest_block_hash;
        const latestAppHash = tenderdashStatus.sync_info.latest_app_hash;

        const platformPeers = parseInt(tenderdashNetInfo.n_peers, 10);
        const { listening } = tenderdashNetInfo;

        info.version = version;
        info.listening = listening;
        info.latestBlockHeight = latestBlockHeight;
        info.latestBlockTime = latestBlockTime;
        info.latestBlockHash = latestBlockHash;
        info.latestAppHash = latestAppHash;
        info.catchingUp = catchingUp;
        info.peers = platformPeers;
        info.moniker = moniker;
        info.network = network;
      } catch (e) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.error('Could not retrieve status from tenderdash RPC', e);
        }

        info.serviceStatus = ServiceStatusEnum.error;
      }
    }

    return info;
  }

  const getDriveInfo = async (config, isCoreSynced) => {
    const info = {
      dockerStatus: null,
      serviceStatus: null,
    };

    try {
      info.dockerStatus = await determineStatus.docker(dockerCompose, configFile, config, 'drive_abci');
      info.serviceStatus = determineStatus.platform(info.dockerStatus, isCoreSynced);

      if (info.serviceStatus === ServiceStatusEnum.up) {
        const driveEchoResult = await dockerCompose.execCommand(
          generateEnvs(configFile, config),
          'drive_abci',
          'drive-abci status',
        );

        if (driveEchoResult.exitCode !== 0) {
          info.serviceStatus = ServiceStatusEnum.error;
        }
      }

      return info;
    } catch (e) {
      if (e instanceof ContainerIsNotPresentError) {
        return {
          dockerStatus: DockerStatusEnum.not_started,
          serviceStatus: ServiceStatusEnum.stopped,
        };
      }

      return info;
    }
  };

  /**
   * Get platform status scope
   *
   * @typedef {Promise} getPlatformScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getPlatformScope(config) {
    const httpPort = config.get('platform.dapi.envoy.http.port');
    const httpService = config.get('externalIp') ? `${config.get('externalIp')}:${httpPort}` : null;
    const p2pPort = config.get('platform.drive.tenderdash.p2p.port');
    const p2pService = config.get('externalIp') ? `${config.get('externalIp')}:${p2pPort}` : null;
    const rpcPort = config.get('platform.drive.tenderdash.rpc.port');
    const rpcService = rpcPort ? `127.0.0.1:${rpcPort}` : rpcPort;

    const scope = {
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

    if (!config.get('platform.enable')) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('Platform is not supported for this node type and network');

        return scope;
      }
    }

    try {
      const coreIsSynced = await getMNSync(config);
      scope.coreIsSynced = coreIsSynced;

      if (!coreIsSynced) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.error('Platform status is not available until masternode state is \'READY\'');
        }
      }
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('Could not get MNSync from core', e);
      }
    }

    const [tenderdash, drive] = await Promise.all([
      getTenderdashInfo(config, scope.coreIsSynced),
      getDriveInfo(config, scope.coreIsSynced),
    ]);

    if (tenderdash) {
      scope.tenderdash = tenderdash;

      scope.httpPortState = tenderdash.httpPortState;
      scope.p2pPortState = tenderdash.p2pPortState;
    }

    if (drive) {
      scope.drive = drive;
    }

    return scope;
  }

  return getPlatformScope;
}

module.exports = getPlatformScopeFactory;
