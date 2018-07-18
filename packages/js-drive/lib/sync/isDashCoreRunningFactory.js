/**
 * Build isDashCoreRunning function
 *
 * @param {RpcClient} rpcClient
 * @return {isDashCoreRunning}
 */
module.exports = function isDashCoreRunningFactory(rpcClient) {
  /**
   * Check is Dash Core running
   *
   * @typedef isDashCoreRunning
   * @return {Promise<boolean>}
   */
  async function isDashCoreRunning() {
    try {
      await rpcClient.ping();

      return true;
    } catch (e) {
      return false;
    }
  }

  return isDashCoreRunning;
};
