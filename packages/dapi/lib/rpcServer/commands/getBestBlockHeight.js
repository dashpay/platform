/**
 * @param coreAPI
 * @return {getBestBlockHeight}
 */
const getBestBlockHeightFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns best block height
   * @typedef getBestBlockHeight
   * @return {Promise<number>} - best seen block height
   */
  async function getBestBlockHeight() {
    return coreAPI.getBestBlockHeight();
  }

  return getBestBlockHeight;
};

module.exports = getBestBlockHeightFactory;
