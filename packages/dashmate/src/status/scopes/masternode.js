const getPaymentQueuePosition = require('../../util/getPaymentQueueposition');
const blocksToTime = require('../../util/blocksToTime');
const MasternodeStateEnum = require('../../enums/masternodeState');
const MasternodeSyncAssetEnum = require('../../enums/masternodeSyncAsset');

module.exports = async (createRpcClient, dockerCompose, config) => {
  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: config.get('core.rpc.user'),
    pass: config.get('core.rpc.password'),
  })

  const mnsyncStatus = await rpcClient.mnsync('status')
  const {AssetName: syncAsset} = mnsyncStatus.result;

  const masternode = {
    proTxHash: null,
    state: null,
    sentinel: {
      state: null,
      version: null
    },
    nodeState: {
      dmnState: null,
      poSePenalty: null,
      lastPaidHeight: null,
      lastPaidTime: null,
      paymentQueuePosition: null,
      nextPaymentTime: null,
    }
  }

  // cannot be put in Promise.all, because sentinel will cause exit 1 with simultaneous requests
  try {
    const sentinelStateResponse = await dockerCompose
      .execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py')
    const sentinelVersionResponse = await dockerCompose
      .execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py -v')

    masternode.sentinel.state = sentinelStateResponse.out.split(/\r?\n/)[0]
    masternode.sentinel.version = sentinelVersionResponse.out.replace(/Dash Sentinel v/, '')
  } catch (e) {
  }


  if (syncAsset === MasternodeSyncAssetEnum.MASTERNODE_SYNC_FINISHED) {
    const [blockchainInfo, masternodeCount, masternodeStatus] =
      await Promise.all([
        rpcClient.getBlockchainInfo(),
        rpcClient.masternode('count'),
        rpcClient.masternode('status'),
      ])

    const {blocks: coreBlocks} = blockchainInfo.result;
    const {dmnState, state, status, proTxHash} = masternodeStatus.result;

    const countInfo = masternodeCount.result;
    const {enabled} = countInfo;

    masternode.sentinel = proTxHash
    masternode.proTxHash = proTxHash
    masternode.status = status
    masternode.state = state

    if (masternodeStatus === MasternodeStateEnum.READY) {
      const position = getPaymentQueuePosition(dmnState, enabled, coreBlocks);

      const poSePenalty = dmnState.PoSePenalty;
      const {lastPaidHeight} = dmnState;
      const lastPaidTime = blocksToTime(coreBlocks - dmnState.lastPaidHeight);
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
};
