const getPaymentQueuePosition = require("../../util/getPaymentQueuePosition");
const blocksToTime = require("../../util/blocksToTime");
const MasternodeStateEnum = require("../../enums/masternodeState");

module.exports = async (coreService, dockerCompose, config) => {
  const sentinelState = (await dockerCompose.execCommand(
    config.toEnvs(),
    'sentinel',
    'python bin/sentinel.py',
  )).out.split(/\r?\n/)[0];

  const sentinelVersion = (await dockerCompose.execCommand(
    config.toEnvs(),
    'sentinel',
    'python bin/sentinel.py -v',
  )).out.split(/\r?\n/)[0].replace(/Dash Sentinel v/, '');

  let position = 0;

  const blockchainInfo = await coreService.getRpcClient().getBlockchainInfo();
  const {blocks: coreBlocks} = blockchainInfo.result

  const masternodeStatus = await coreService.getRpcClient().masternode('status');
  const {dmnState, state, status, proTxHash} = masternodeStatus.result

  const countInfo = await coreService.getRpcClient().masternode('count')
  const {enabled} = countInfo.result

  const nodeState = {
    dmnState: null,
    poSePenalty: null,
    lastPaidHeight: null,
    lastPaidTime: null,
    paymentQueuePosition: null,
    nextPaymentTime: null
  }

  if (masternodeStatus === MasternodeStateEnum.READY) {
    position = getPaymentQueuePosition(dmnState, enabled, coreBlocks);

    const poSePenalty = dmnState.PoSePenalty;
    const lastPaidHeight = dmnState.lastPaidHeight;
    const lastPaidTime = blocksToTime(coreBlocks - dmnState.lastPaidHeight);
    const paymentQueuePosition = position / enabled;
    const nextPaymentTime = `${blocksToTime(paymentQueuePosition)}`;

    nodeState.dmnState = dmnState
    nodeState.poSePenalty = poSePenalty
    nodeState.lastPaidHeight = lastPaidHeight
    nodeState.lastPaidTime = lastPaidTime
    nodeState.paymentQueuePosition = paymentQueuePosition
    nodeState.nextPaymentTime = nextPaymentTime
  }

  return {
    status,
    state,
    enabledCount: enabled,
    proTxHash,
    sentinelState,
    sentinelVersion,
    nodeState
  }
}
