const {
  v0: {
    GetStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @returns {getStatusHandler}
 */
function getStatusHandlerFactory(coreRPCClient) {
  /**
   * @typedef getStatusHandler
   * @return {Promise<GetStatusResponse>}
   */
  async function getStatusHandler() {
    const [
      blockchainInfoResponse,
      networkInfoResponse,
      mnSyncStatusResponse,
      masternodeStatusResponse,
    ] = await Promise.all([
      coreRPCClient.getBlockchainInfo(),
      coreRPCClient.getNetworkInfo(),
      coreRPCClient.getMnSync('status'),
      coreRPCClient.getMasternode('status'),
    ]);

    const response = new GetStatusResponse();

    const version = new GetStatusResponse.Version();
    version.setProtocol(networkInfoResponse.protocolversion);
    version.setSoftware(networkInfoResponse.version);
    version.setAgent(networkInfoResponse.subversion);

    const time = new GetStatusResponse.Time();
    time.setNow(Math.floor(Date.now() / 1000));
    time.setOffset(networkInfoResponse.timeoffset);
    time.setMedian(blockchainInfoResponse.mediantime);

    const chain = new GetStatusResponse.Chain();
    chain.setName(blockchainInfoResponse.chain);
    chain.setBlocksCount(blockchainInfoResponse.blocks);
    chain.setHeadersCount(blockchainInfoResponse.headers);
    chain.setBestBlockHash(blockchainInfoResponse.bestblockhash);
    chain.setDifficulty(blockchainInfoResponse.difficulty);
    chain.setChainWork(blockchainInfoResponse.chainwork);
    chain.setIsSynced(mnSyncStatusResponse.IsBlockchainSynced);
    chain.setSyncProgress(blockchainInfoResponse.verificationprogress);

    const masternode = new GetStatusResponse.Masternode();

    const masternodeStatus = GetStatusResponse.Masternode.Status[masternodeStatusResponse.state];

    masternode.setStatus(masternodeStatus);
    masternode.setProTxHash(masternodeStatusResponse.proTxHash);
    masternode.setPosePenalty(masternodeStatusResponse.dmnState.PoSePenalty);
    masternode.setIsSynced(mnSyncStatusResponse.IsSynced);

    let syncProgress;
    switch (mnSyncStatusResponse.AssetID) {
      case 999:
        syncProgress = 1;
        break;
      case 0:
        syncProgress = 0;
        break;
      case 1:
        syncProgress = 1 / 3;
        break;
      case 4:
        syncProgress = 2 / 3;
        break;
      default:
        syncProgress = 0;
    }

    masternode.setSyncProgress(syncProgress);

    const network = new GetStatusResponse.Network();
    network.setPeersCount(networkInfoResponse.connections);

    const networkFee = new GetStatusResponse.NetworkFee();
    networkFee.setRelay(networkInfoResponse.relayfee);
    networkFee.setIncremental(networkInfoResponse.incrementalfee);

    network.setFee(networkFee);

    response.setVersion(version);
    response.setTime(time);
    response.setSyncProgress(blockchainInfoResponse.verificationprogress);
    response.setChain(chain);
    response.setMasternode(masternode);
    response.setNetwork(network);

    let status = GetStatusResponse.Status.NOT_STARTED;
    if (mnSyncStatusResponse.IsBlockchainSynced && mnSyncStatusResponse.IsSynced) {
      status = GetStatusResponse.Status.READY;
    } else if (blockchainInfoResponse.verificationprogress > 0) {
      status = GetStatusResponse.Status.SYNCING;
    }

    response.setStatus(status);

    return response;
  }

  return getStatusHandler;
}

module.exports = getStatusHandlerFactory;
