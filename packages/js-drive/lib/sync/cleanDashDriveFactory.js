/**
 *
 * @param unpinAllIpfsPackets
 * @param dropDriveMongoDatabases
 * @returns {cleanDashDrive}
 */
function cleanDashDriveFactory(unpinAllIpfsPackets, dropDriveMongoDatabases) {
  /**
   * Cleanup DashDrive IPFS packets and MongoDB databases
   *
   * @typedef {Promise} cleanDashDrive
   * @param {string} mongoDbPrefix
   * @returns {Promise<void>}
   */
  async function cleanDashDrive(mongoDbPrefix) {
    await unpinAllIpfsPackets();
    await dropDriveMongoDatabases(mongoDbPrefix);
  }

  return cleanDashDrive;
}

module.exports = cleanDashDriveFactory;
