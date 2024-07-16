/* eslint-disable camelcase */
import providers from '../providers.js';
import { ServiceStatusEnum } from '../enums/serviceStatus.js';
import { DockerStatusEnum } from '../enums/dockerStatus.js';
import determineStatus from '../determineStatus.js';
import extractCoreVersion from '../../core/extractCoreVersion.js';

/**
 * @returns {getCoreScopeFactory}
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 */
export default function getCoreScopeFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
) {
  /*
   * Get core status scope
   *
   * @typedef {Promise} getCoreScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getCoreScope(config) {
    const network = config.get('network');
    const rpcService = `${config.get('core.rpc.host')}:${config.get('core.rpc.port')}`;
    const p2pService = config.get('externalIp') ? `${config.get('externalIp')}:${config.get('core.p2p.port')}` : null;

    const core = {
      network,
      rpcService,
      p2pService,
      version: null,
      chain: null,
      latestVersion: null,
      dockerStatus: null,
      serviceStatus: null,
      peersCount: null,
      p2pPortState: null,
      blockHeight: null,
      remoteBlockHeight: null,
      headerHeight: null,
      difficulty: null,
      verificationProgress: null,
      sizeOnDisk: null,
      syncAsset: null,
    };

    // this try catch handle getConnectionHost, isNodeRunning calls
    try {
      if (!(await dockerCompose.isServiceRunning(config, 'core'))) {
        core.dockerStatus = DockerStatusEnum.not_started;
        core.serviceStatus = ServiceStatusEnum.stopped;

        return core;
      }

      core.dockerStatus = await determineStatus.docker(dockerCompose, config, 'core');

      if (core.dockerStatus !== DockerStatusEnum.running) {
        core.serviceStatus = ServiceStatusEnum.error;

        return core;
      }
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('Could not fetch docker status for core', e);
      }

      return core;
    }

    try {
      const rpcClient = createRpcClient({
        port: config.get('core.rpc.port'),
        user: 'dashmate',
        pass: config.get('core.rpc.users.dashmate.password'),
        host: await getConnectionHost(config, 'core', 'core.rpc.host'),
      });

      const [mnsyncStatus, networkInfo, blockchainInfo] = await Promise.all([
        rpcClient.mnsync('status'),
        rpcClient.getNetworkInfo(),
        rpcClient.getBlockchainInfo(),
      ]);

      const { AssetName: syncAsset } = mnsyncStatus.result;
      core.serviceStatus = determineStatus.core(core.dockerStatus, syncAsset);
      core.syncAsset = syncAsset;

      const {
        chain, difficulty, blocks, headers, verificationprogress, size_on_disk,
      } = blockchainInfo.result;

      core.chain = chain;
      core.difficulty = difficulty;
      core.blockHeight = blocks;
      core.headerHeight = headers;
      core.verificationProgress = verificationprogress;
      core.sizeOnDisk = size_on_disk;

      const { subversion, connections } = networkInfo.result;

      core.peersCount = connections;
      core.version = extractCoreVersion(subversion);
    } catch (e) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error('Could not fetch dashcore RPC', e);
      }
      core.serviceStatus = ServiceStatusEnum.error;
    }

    const providersResult = await Promise.allSettled([
      providers.github.release('dashpay/dash'),
      providers.mnowatch.checkPortStatus(config.get('core.p2p.port')),
      providers.insight(config.get('network')).status(),
    ]);

    if (process.env.DEBUG) {
      for (const error of providersResult.filter((e) => e.status === 'rejected')) {
        // eslint-disable-next-line no-console
        console.error('Could not retrieve provider response', error.reason);
      }
    }

    const [latestVersion, p2pPortState, insightStatus] = providersResult
      .map((result) => (result.status === 'fulfilled' ? result.value : null));

    core.latestVersion = latestVersion;
    core.p2pPortState = p2pPortState;
    core.remoteBlockHeight = insightStatus ? insightStatus.info.blocks : null;

    return core;
  }

  return getCoreScope;
}
