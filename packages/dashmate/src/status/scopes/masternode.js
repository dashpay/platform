const getPaymentQueuePosition = require('../../core/getPaymentQueuePosition');
const blocksToTime = require('../../util/blocksToTime');
const MasternodeStateEnum = require('../../enums/masternodeState');
const MasternodeSyncAssetEnum = require('../../enums/masternodeSyncAsset');

/**
 * @returns {getMasternodeScopeFactory}
 * @param dockerCompose {DockerCompose}
 * @param createRpcClient {createRpcClient}
 */
function getMasternodeScopeFactory(dockerCompose, createRpcClient) {
  /**
   * Get masternode status scope
   *
   * @typedef {Promise} getMasternodeScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getMasternodeScope(config) {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
    });

    const mnsyncStatus = await rpcClient.mnsync('status');
    const { AssetName: syncAsset } = mnsyncStatus.result;

    const masternode = {
      syncAsset,
      sentinel: {
        state: null,
        version: null,
      },
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

    // cannot be put in Promise.all, because sentinel will cause exit 1 with simultaneous requests
    try {
      const sentinelStateResponse = await dockerCompose
        .execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py');
      const sentinelVersionResponse = await dockerCompose
        .execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py -v');

      const [state] = sentinelStateResponse.out.split(/\r?\n/);

      masternode.sentinel.state = state;
      masternode.sentinel.version = sentinelVersionResponse.out.replace(/Dash Sentinel v/, '');
      // eslint-disable-next-line no-empty
    } catch (e) {
    }

    if (syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
      const [blockchainInfo, masternodeCount, masternodeStatus] = await Promise.all([
        rpcClient.getBlockchainInfo(),
        rpcClient.masternode('count'),
        rpcClient.masternode('status'),
      ]);

      const { blocks: coreBlocks } = blockchainInfo.result;

      const countInfo = masternodeCount.result;
      const { enabled } = countInfo;

      const { state, status, proTxHash } = masternodeStatus.result;

      masternode.proTxHash = proTxHash;
      masternode.status = status;
      masternode.state = state;

      if (state === MasternodeStateEnum.READY) {
        const { dmnState } = masternodeStatus.result;

        const { PoSePenalty: poSePenalty, lastPaidHeight } = dmnState;

        const position = getPaymentQueuePosition(dmnState, enabled, coreBlocks);
        const lastPaidTime = blocksToTime(coreBlocks - lastPaidHeight);
        const paymentQueuePosition = position / enabled;
        const nextPaymentTime = `${blocksToTime(paymentQueuePosition)}`;

        masternode.nodeState.dmnState = dmnState;
        masternode.nodeState.poSePenalty = poSePenalty;
        masternode.nodeState.lastPaidHeight = lastPaidHeight;
        masternode.nodeState.lastPaidTime = lastPaidTime;
        masternode.nodeState.paymentQueuePosition = paymentQueuePosition;
        masternode.nodeState.nextPaymentTime = nextPaymentTime;
      }
    }

    return masternode;
  }

  return getMasternodeScope;
}

module.exports = getMasternodeScopeFactory;
