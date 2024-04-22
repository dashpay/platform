const {
  v0: {
    GetCoreChainStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @returns {getCoreChainStatusHandler}
 */
function getCoreChainStatusHandlerFactory(coreRPCClient) {
  /**
   * @typedef getCoreChainStatusHandler
   * @return {Promise<GetCoreChainStatusResponse>}
   */
  async function getCoreChainStatusHandler() {
    const [
      blockchainInfoResponse,
      networkInfoResponse,
    ] = await Promise.all([
      coreRPCClient.getBlockchainInfo(),
      coreRPCClient.getNetworkInfo(),
    ]);

    const response = new GetCoreChainStatusResponse();

    const version = new GetCoreChainStatusResponse.Version();
    version.setProtocol(networkInfoResponse.protocolversion);
    version.setSoftware(networkInfoResponse.version);
    version.setAgent(networkInfoResponse.subversion);

    const time = new GetCoreChainStatusResponse.Time();
    time.setNow(Math.floor(Date.now() / 1000));
    time.setOffset(networkInfoResponse.timeoffset);
    time.setMedian(blockchainInfoResponse.mediantime);

    const chain = new GetCoreChainStatusResponse.Chain();
    chain.setName(blockchainInfoResponse.chain);
    chain.setBlocksCount(blockchainInfoResponse.blocks);
    chain.setHeadersCount(blockchainInfoResponse.headers);
    chain.setBestBlockHash(Buffer.from(blockchainInfoResponse.bestblockhash, 'hex'));
    chain.setDifficulty(blockchainInfoResponse.difficulty);
    chain.setChainWork(Buffer.from(blockchainInfoResponse.chainwork, 'hex'));
    chain.setIsSynced(blockchainInfoResponse.verificationprogress === 1);
    chain.setSyncProgress(blockchainInfoResponse.verificationprogress);

    const network = new GetCoreChainStatusResponse.Network();
    network.setPeersCount(networkInfoResponse.connections);

    const networkFee = new GetCoreChainStatusResponse.NetworkFee();
    networkFee.setRelay(networkInfoResponse.relayfee);
    networkFee.setIncremental(networkInfoResponse.incrementalfee);

    network.setFee(networkFee);

    response.setVersion(version);
    response.setTime(time);
    response.setSyncProgress(blockchainInfoResponse.verificationprogress);
    response.setChain(chain);
    response.setNetwork(network);

    let status = GetCoreChainStatusResponse.Status.SYNCING;
    if (blockchainInfoResponse.verificationprogress === 1) {
      status = GetCoreChainStatusResponse.Status.READY;
    }

    response.setStatus(status);

    return response;
  }

  return getCoreChainStatusHandler;
}

module.exports = getCoreChainStatusHandlerFactory;
