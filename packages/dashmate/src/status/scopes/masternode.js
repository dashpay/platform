const calculatePaymentQueuePosition = require('../../core/calculatePaymentQueuePosition');
const blocksToTime = require('../../util/blocksToTime');
const MasternodeStateEnum = require('../enums/masternodeState');
const MasternodeSyncAssetEnum = require('../enums/masternodeSyncAsset');
const providers = require("../providers");

/**
 * @returns {getMasternodeScopeFactory}
 * @param dockerCompose {DockerCompose}
 * @param createRpcClient {createRpcClient}
 * @param getConnectionHost {getConnectionHost}
 */
function getMasternodeScopeFactory(dockerCompose, createRpcClient, getConnectionHost) {
  async function getMNSync() {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
      host: await getConnectionHost(config, 'core'),
    });

    const mnsyncStatus = await rpcClient.mnsync('status');
    const {AssetName: syncAsset} = mnsyncStatus.result;

    return syncAsset
  }

  async function getMasternodeInfo() {
    const rpcClient = createRpcClient({
      port: config.get('core.rpc.port'),
      user: config.get('core.rpc.user'),
      pass: config.get('core.rpc.password'),
      host: await getConnectionHost(config, 'core'),
    });

    const masternodeInfo = {nodeState: {}}

    const [blockchainInfo, masternodeCount, masternodeStatus] = await Promise.all([
      rpcClient.getBlockchainInfo(),
      rpcClient.masternode('count'),
      rpcClient.masternode('status'),
    ]);
    const {blocks: coreBlocks} = blockchainInfo.result;

    const countInfo = masternodeCount.result;
    const {enabled} = countInfo;

    const {state, status, proTxHash} = masternodeStatus.result;

    masternodeInfo.proTxHash = proTxHash;
    masternodeInfo.status = status;
    masternodeInfo.state = state;

    if (state === MasternodeStateEnum.READY) {
      const {dmnState} = masternodeStatus.result;

      const {PoSePenalty: poSePenalty, lastPaidHeight} = dmnState;

      const position = calculatePaymentQueuePosition(dmnState, enabled, coreBlocks);
      const lastPaidTime = blocksToTime(coreBlocks - lastPaidHeight);
      const paymentQueuePosition = position / enabled;
      const nextPaymentTime = `${blocksToTime(paymentQueuePosition)}`;

      masternodeInfo.nodeState.dmnState = dmnState;
      masternodeInfo.nodeState.poSePenalty = poSePenalty;
      masternodeInfo.nodeState.lastPaidHeight = lastPaidHeight;
      masternodeInfo.nodeState.lastPaidTime = lastPaidTime;
      masternodeInfo.nodeState.paymentQueuePosition = paymentQueuePosition;
      masternodeInfo.nodeState.nextPaymentTime = nextPaymentTime;
    }

    return masternodeInfo
  }

  async function getSentinelInfo() {
    // cannot be put in Promise.all, because sentinel will cause exit 1 with simultaneous requests
    const sentinelStateResponse = await dockerCompose
      .execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py');
    const sentinelVersionResponse = await dockerCompose
      .execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py -v');

    const [state] = sentinelStateResponse.out.split(/\r?\n/);

    return {
      state: state === '' ? 'ok' : state,
      version: sentinelVersionResponse.out
        .replace(/Dash Sentinel v/, '').trim()
    }
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
      sentinel: {
        state: null,
        version: null,
      },
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
      getMNSync(),
      getSentinelInfo(),
    ]);

    const [mnSync, sentinelInfo] = basicResult
      .map((result) => (result.status === 'fulfilled' ? result.value : null));

    if (mnSync) {
      scope.syncAsset = mnSync
    }

    if (sentinelInfo) {
      scope.sentinel.state = sentinelInfo.state
      scope.sentinel.version = sentinelInfo.version
    }

    if (scope.syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
      try {
        const masternodeInfo = await getMasternodeInfo()

        scope.proTxHash = masternodeInfo.proTxHash
        scope.state = masternodeInfo.state
        scope.status = masternodeInfo.status
        scope.nodeState = masternodeInfo.nodeState
      } catch (e) {
      }
    }

    return scope;
  }

  return getMasternodeScope;
}

module.exports = getMasternodeScopeFactory;
