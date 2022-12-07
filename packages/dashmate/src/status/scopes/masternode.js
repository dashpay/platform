const getPaymentQueuePosition = require('../../util/getPaymentQueueposition');
const blocksToTime = require('../../util/blocksToTime');
const MasternodeStateEnum = require('../../enums/masternodeState');

module.exports = async (createRpcClient, dockerCompose, config) => {
  const rpcClient = createRpcClient({
    port: config.get('core.rpc.port'),
    user: config.get('core.rpc.user'),
    pass: config.get('core.rpc.password'),
  })

  const [sentinelStateResponse, sentinelVersionResponse, blockchainInfo, masternodeStatus, masternodeCount] =
    await Promise.all([
      dockerCompose.execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py',),
      dockerCompose.execCommand(config.toEnvs(), 'sentinel', 'python bin/sentinel.py -v'),
      rpcClient.getBlockchainInfo(),
      rpcClient.masternode('status'),
      rpcClient.masternode('count')
    ])

  const sentinelState = sentinelStateResponse.out.split(/\r?\n/)[0]
  const sentinelVersion = sentinelVersionResponse.replace(/Dash Sentinel v/, '')

  const {blocks: coreBlocks} = blockchainInfo.result;
  const {
    dmnState, state, status, proTxHash,
  } = masternodeStatus.result;

  const countInfo = await rpcClient.masternode('count');
  const {enabled} = countInfo.result;

  const nodeState = {
    dmnState: null,
    poSePenalty: null,
    lastPaidHeight: null,
    lastPaidTime: null,
    paymentQueuePosition: null,
    nextPaymentTime: null,
  };

  if (masternodeStatus === MasternodeStateEnum.READY) {
    const position = getPaymentQueuePosition(dmnState, enabled, coreBlocks);

    const poSePenalty = dmnState.PoSePenalty;
    const {lastPaidHeight} = dmnState;
    const lastPaidTime = blocksToTime(coreBlocks - dmnState.lastPaidHeight);
    const paymentQueuePosition = position / enabled;
    const nextPaymentTime = `${blocksToTime(paymentQueuePosition)}`;

    nodeState.dmnState = dmnState;
    nodeState.poSePenalty = poSePenalty;
    nodeState.lastPaidHeight = lastPaidHeight;
    nodeState.lastPaidTime = lastPaidTime;
    nodeState.paymentQueuePosition = paymentQueuePosition;
    nodeState.nextPaymentTime = nextPaymentTime;
  }

  return {
    status,
    state,
    enabledCount: enabled,
    proTxHash,
    sentinelState,
    sentinelVersion,
    nodeState,
  };
};
