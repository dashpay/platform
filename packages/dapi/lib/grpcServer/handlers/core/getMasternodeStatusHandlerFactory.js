const {
  v0: {
    GetMasternodeStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {CoreRpcClient} coreRPCClient
 * @returns {getMasternodeStatusHandler}
 */
function getMasternodeStatusHandlerFactory(coreRPCClient) {
  /**
   * @typedef getMasternodeStatusHandler
   * @return {Promise<GetMasternodeStatusResponse>}
   */
  async function getMasternodeStatusHandler() {
    const [
      mnSyncStatusResponse,
      masternodeStatusResponse,
    ] = await Promise.all([
      coreRPCClient.getMnSync('status'),
      coreRPCClient.getMasternode('status'),
    ]);

    const response = new GetMasternodeStatusResponse();

    const masternodeStatus = GetMasternodeStatusResponse.Status[masternodeStatusResponse.state];

    response.setStatus(masternodeStatus);

    if (masternodeStatusResponse.proTxHash) {
      response.setProTxHash(Buffer.from(masternodeStatusResponse.proTxHash, 'hex'));
    }

    if (masternodeStatusResponse.dmnState) {
      response.setPosePenalty(masternodeStatusResponse.dmnState.PoSePenalty);
    }

    response.setIsSynced(mnSyncStatusResponse.IsSynced);

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

    response.setSyncProgress(syncProgress);

    return response;
  }

  return getMasternodeStatusHandler;
}

module.exports = getMasternodeStatusHandlerFactory;
