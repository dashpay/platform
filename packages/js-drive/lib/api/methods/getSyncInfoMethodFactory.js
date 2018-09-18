/**
 * @param {getSyncInfo} getSyncInfo
 * @returns {getSyncInfoMethod}
 */
function getSyncInfoMethodFactory(getSyncInfo) {
  /**
   * @typedef getSyncInfoMethod
   * @returns {Promise<Object>}
   */
  async function getSyncInfoMethod() {
    try {
      const syncInfo = await getSyncInfo();
      return syncInfo.toJSON();
    } catch (error) {
      throw error;
    }
  }

  return getSyncInfoMethod;
}

module.exports = getSyncInfoMethodFactory;
