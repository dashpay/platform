const calculatePaymentQueuePosition = require('../../core/calculatePaymentQueuePosition');
const blocksToTime = require('../../util/blocksToTime');
const MasternodeStateEnum = require('../enums/masternodeState');
const MasternodeSyncAssetEnum = require('../enums/masternodeSyncAsset');

/**
 * @param {DockerCompose}dockerCompose
 * @param {createRpcClient} createRpcClient
 * @param {getConnectionHost} getConnectionHost
 * @returns {getMasternodeScopeFactory}
 */
function getMasternodeScopeFactory(dockerCompose, createRpcClient, getConnectionHost) {
  async function getSyncAsset(config) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
      host: await getConnectionHost(config, 'core'),
    });

    const mnsyncStatus = await rpcClient.mnsync('status');
    const { AssetName: syncAsset } = mnsyncStatus.result;

    return syncAsset;
  }

  async function getMasternodeInfo(config) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
      host: await getConnectionHost(config, 'core'),
    });

    const info = {
      proTxHash: null,
      state: null,
      status: null,
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
    const { enabled } = countInfo;

    const { state, status, proTxHash } = masternodeStatus.result;

    info.proTxHash = proTxHash;
    info.status = status;
    info.state = state;

    if (state === MasternodeStateEnum.READY) {
      const { dmnState } = masternodeStatus.result;

      const { PoSePenalty: poSePenalty, lastPaidHeight } = dmnState;

      const position = calculatePaymentQueuePosition(dmnState, enabled, coreBlocks);
      const lastPaidTime = blocksToTime(coreBlocks - lastPaidHeight);
      const paymentQueuePosition = position / enabled;
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
      nodeState: {
        dmnState: null,
        poSePenalty: null,
        lastPaidHeight: null,
        lastPaidTime: null,
        paymentQueuePosition: null,
        nextPaymentTime: null,
      },
    };

    const basicResult = await Promise.allSettled([
      getSyncAsset(config),
    ]);

    if (process.env.DEBUG) {
      for (const error of basicResult.filter((e) => e.status === 'rejected')) {
        // eslint-disable-next-line no-console
        console.error(error.reason);
      }
    }

    const [syncAsset] = basicResult
      .map((result) => (result.status === 'fulfilled' ? result.value : null));

    if (syncAsset) {
      scope.syncAsset = syncAsset;
    }

    if (scope.syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
      try {
        const masternodeInfo = await getMasternodeInfo(config);

        scope.proTxHash = masternodeInfo.proTxHash;
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

module.exports = getMasternodeScopeFactory;
