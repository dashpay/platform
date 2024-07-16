import calculatePaymentQueuePosition from '../../core/calculatePaymentQueuePosition.js';
import { MasternodeSyncAssetEnum } from '../enums/masternodeSyncAsset.js';
import blocksToTime from '../../util/blocksToTime.js';
import { MasternodeStateEnum } from '../enums/masternodeState.js';

/**
 * @param {DockerCompose}dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @returns {getMasternodeScopeFactory}
 */
export default function getMasternodeScopeFactory(
  dockerCompose,
  createRpcClient,
  getConnectionHost,
) {
  async function getSyncAsset(config) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: 'dashmate',
      pass: config.get('core.rpc.users.dashmate.password'),
      host: await getConnectionHost(config, 'core', 'core.rpc.host'),
    });

    const mnsyncStatus = await rpcClient.mnsync('status');
    const { AssetName: syncAsset } = mnsyncStatus.result;

    return syncAsset;
  }

  async function getMasternodeInfo(config) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: 'dashmate',
      pass: config.get('core.rpc.users.dashmate.password'),
      host: await getConnectionHost(config, 'core', 'core.rpc.host'),
    });

    const info = {
      proTxHash: null,
      state: null,
      status: null,
      masternodeTotal: null,
      masternodeEnabled: null,
      evonodeTotal: null,
      evonodeEnabled: null,
      nodeState: {
        dmnState: null,
        poSePenalty: null,
        lastPaidHeight: null,
        lastPaidTime: null,
        paymentQueuePosition: null,
        nextPaymentTime: null,
      },
    };

    const [blockchainInfo, masternodeCount, masternodeStatus] = await Promise.all([
      rpcClient.getBlockchainInfo(),
      rpcClient.masternode('count'),
      rpcClient.masternode('status'),
    ]);
    const { blocks: coreBlocks } = blockchainInfo.result;

    const countInfo = masternodeCount.result;
    const { detailed } = countInfo;
    const { regular, evo } = detailed;

    info.masternodeTotal = regular.total;
    info.masternodeEnabled = regular.enabled;
    info.evonodeTotal = evo.total;
    info.evonodeEnabled = evo.enabled;

    const { state, status, proTxHash } = masternodeStatus.result;

    info.proTxHash = proTxHash;
    info.status = status;
    info.state = state;

    if (state === MasternodeStateEnum.READY) {
      const { dmnState } = masternodeStatus.result;

      const { PoSePenalty: poSePenalty, lastPaidHeight } = dmnState;

      const paymentQueuePosition = calculatePaymentQueuePosition(
        dmnState,
        info.masternodeEnabled,
        info.evonodeEnabled,
        coreBlocks,
      );
      const lastPaidTime = lastPaidHeight ? blocksToTime(coreBlocks - lastPaidHeight) : null;
      const nextPaymentTime = `${blocksToTime(paymentQueuePosition)}`;

      info.nodeState.dmnState = dmnState;
      info.nodeState.poSePenalty = poSePenalty;
      info.nodeState.lastPaidHeight = lastPaidHeight;
      info.nodeState.lastPaidTime = lastPaidTime;
      info.nodeState.paymentQueuePosition = paymentQueuePosition;
      info.nodeState.nextPaymentTime = nextPaymentTime;
    }

    return info;
  }

  /**
   * Get masternode status scope
   *
   * @typedef {Promise} getMasternodeScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getMasternodeScope(config) {
    const scope = {
      syncAsset: null,
      proTxHash: null,
      state: MasternodeStateEnum.UNKNOWN,
      status: null,
      masternodeTotal: null,
      masternodeEnabled: null,
      evonodeTotal: null,
      evonodeEnabled: null,
      nodeState: {
        dmnState: null,
        poSePenalty: null,
        lastPaidHeight: null,
        lastPaidTime: null,
        paymentQueuePosition: null,
        nextPaymentTime: null,
      },
    };

    try {
      scope.syncAsset = await getSyncAsset(config);
    } catch (error) {
      if (process.env.DEBUG) {
        // eslint-disable-next-line no-console
        console.error(error);
      }
    }

    if (scope.syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
      try {
        const masternodeInfo = await getMasternodeInfo(config);

        scope.proTxHash = masternodeInfo.proTxHash;
        scope.masternodeTotal = masternodeInfo.masternodeTotal;
        scope.masternodeEnabled = masternodeInfo.masternodeEnabled;
        scope.evonodeEnabled = masternodeInfo.evonodeEnabled;
        scope.evonodeTotal = masternodeInfo.evonodeTotal;
        scope.state = masternodeInfo.state;
        scope.status = masternodeInfo.status;
        scope.nodeState = masternodeInfo.nodeState;
      } catch (e) {
        if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.error('Could not retrieve dashcore masternode info', e);
        }
      }
    }

    return scope;
  }

  return getMasternodeScope;
}
