const {
  v0: {
    GetBlockchainStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @returns {getBlockchainStatusHandler}
 */
function getBlockchainStatusHandlerFactory(coreRPCClient) {
  /**
   * @typedef getBlockchainStatusHandler
   * @return {Promise<GetBlockchainStatusResponse>}
   */
  async function getBlockchainStatusHandler() {
    const [
      blockchainInfoResponse,
      networkInfoResponse,
    ] = await Promise.all([
      coreRPCClient.getBlockchainInfo(),
      coreRPCClient.getNetworkInfo(),
    ]);

    const response = new GetBlockchainStatusResponse();

    const version = new GetBlockchainStatusResponse.Version();
    version.setProtocol(networkInfoResponse.protocolversion);
    version.setSoftware(networkInfoResponse.version);
    version.setAgent(networkInfoResponse.subversion);

    const time = new GetBlockchainStatusResponse.Time();
    time.setNow(Math.floor(Date.now() / 1000));
    time.setOffset(networkInfoResponse.timeoffset);
    time.setMedian(blockchainInfoResponse.mediantime);

    const chain = new GetBlockchainStatusResponse.Chain();
    chain.setName(blockchainInfoResponse.chain);
    chain.setBlocksCount(blockchainInfoResponse.blocks);
    chain.setHeadersCount(blockchainInfoResponse.headers);
    chain.setBestBlockHash(Buffer.from(blockchainInfoResponse.bestblockhash, 'hex'));
    chain.setDifficulty(blockchainInfoResponse.difficulty);
    chain.setChainWork(Buffer.from(blockchainInfoResponse.chainwork, 'hex'));
    chain.setIsSynced(blockchainInfoResponse.verificationprogress === 1);
    chain.setSyncProgress(blockchainInfoResponse.verificationprogress);

    const network = new GetBlockchainStatusResponse.Network();
    network.setPeersCount(networkInfoResponse.connections);

    const networkFee = new GetBlockchainStatusResponse.NetworkFee();
    networkFee.setRelay(networkInfoResponse.relayfee);
    networkFee.setIncremental(networkInfoResponse.incrementalfee);

    network.setFee(networkFee);

    response.setVersion(version);
    response.setTime(time);
    response.setSyncProgress(blockchainInfoResponse.verificationprogress);
    response.setChain(chain);
    response.setNetwork(network);

    let status = GetBlockchainStatusResponse.Status.SYNCING;
    if (blockchainInfoResponse.verificationprogress === 1) {
      status = GetBlockchainStatusResponse.Status.READY;
    }

    response.setStatus(status);

    return response;
  }

  return getBlockchainStatusHandler;
}

module.exports = getBlockchainStatusHandlerFactory;
