const {
  v0: {
    GetProtocolVersionUpgradeVoteStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getProtocolVersionUpgradeVoteStatusHandler}
 */
function getProtocolVersionUpgradeVoteStatusHandlerFactory(driveClient) {
  /**
   * @typedef getProtocolVersionUpgradeVoteStatusHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetProtocolVersionUpgradeVoteStatusResponse>}
   */
  async function getProtocolVersionUpgradeVoteStatusHandler(call) {
    const { request } = call;

    const versionUpgradeVoteStatusBuffer = await driveClient
      .fetchVersionUpgradeVoteStatus(request);

    return GetProtocolVersionUpgradeVoteStatusResponse
      .deserializeBinary(versionUpgradeVoteStatusBuffer);
  }

  return getProtocolVersionUpgradeVoteStatusHandler;
}

module.exports = getProtocolVersionUpgradeVoteStatusHandlerFactory;
