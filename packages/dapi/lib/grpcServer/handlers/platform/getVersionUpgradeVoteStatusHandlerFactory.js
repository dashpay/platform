const {
  v0: {
    GetVersionUpgradeVoteStatusResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getVersionUpgradeVoteStatusHandler}
 */
function getVersionUpgradeVoteStatusHandlerFactory(driveClient) {
  /**
   * @typedef getVersionUpgradeVoteStatusHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetVersionUpgradeVoteStatusResponse>}
   */
  async function getVersionUpgradeVoteStatusHandler(call) {
    const { request } = call;

    const versionUpgradeVoteStatusBuffer = await driveClient
      .fetchVersionUpgradeVoteStatus(request);

    return GetVersionUpgradeVoteStatusResponse
      .deserializeBinary(versionUpgradeVoteStatusBuffer);
  }

  return getVersionUpgradeVoteStatusHandler;
}

module.exports = getVersionUpgradeVoteStatusHandlerFactory;
