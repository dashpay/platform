/**
 *
 * @param {unpinAllIpfsPackets} unpinAllIpfsPackets
 * @param {dropMongoDatabasesWithPrefix} dropMongoDatabasesWithPrefix
 * @param {string} mongoDbPrefix
 * @returns {cleanDashDrive}
 */
function cleanDashDriveFactory(unpinAllIpfsPackets, dropMongoDatabasesWithPrefix, mongoDbPrefix) {
  /**
   * Cleanup DashDrive IPFS packets and MongoDB databases
   *
   * @typedef {Promise} cleanDashDrive
   * @returns {Promise<void>}
   */
  async function cleanDashDrive() {
    await unpinAllIpfsPackets();
    await dropMongoDatabasesWithPrefix(mongoDbPrefix);
  }

  return cleanDashDrive;
}

module.exports = cleanDashDriveFactory;
