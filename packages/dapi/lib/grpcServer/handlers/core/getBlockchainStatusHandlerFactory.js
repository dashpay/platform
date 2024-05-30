const {
  v0: {
    GetBlockchainStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @param {ZmqClient} coreZmqClient
 * @returns {getBlockchainStatusHandler}
 */
function getBlockchainStatusHandlerFactory(coreRPCClient, coreZmqClient) {
  let response = null;

  // Reset height on a new block, so it will be obtained again on a user request
  coreZmqClient.on(
    coreZmqClient.topics.hashblock,
    () => {
      response = null;
    },
  );

  /**
   * @typedef getBlockchainStatusHandler
   * @return {Promise<GetBlockchainStatusResponse>}
   */
  async function getBlockchainStatusHandler() {
    if (response === null) {
      const [
        blockchainInfoResponse,
        networkInfoResponse,
      ] = await Promise.all([
        coreRPCClient.getBlockchainInfo(),
        coreRPCClient.getNetworkInfo(),
      ]);

      response = new GetBlockchainStatusResponse();

      const version = new GetBlockchainStatusResponse.Version();
      version.setProtocol(networkInfoResponse.protocolversion);
      version.setSoftware(networkInfoResponse.version);
      version.setAgent(networkInfoResponse.subversion);

      const time = new GetBlockchainStatusResponse.Time();
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
    }

    // Set now to current time
    response.getTime().setNow(Math.floor(Date.now() / 1000));

    return response;
  }

  return getBlockchainStatusHandler;
}

module.exports = getBlockchainStatusHandlerFactory;
