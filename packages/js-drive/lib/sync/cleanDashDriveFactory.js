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
   * @returns {Promise<void>}
   */
  async function cleanDashDrive() {
    await unpinAllIpfsPackets();
    await dropDriveMongoDatabases();
  }

  return cleanDashDrive;
}

module.exports = cleanDashDriveFactory;
