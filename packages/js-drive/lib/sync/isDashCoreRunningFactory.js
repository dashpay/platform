const wait = require('../util/wait');

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
   * @param {number} [retries]
   * @param {number} [retryDelay]
   * @returns {Promise<boolean>}
   */
  async function isDashCoreRunning(retries = 1, retryDelay = 5) {
    let attempts = 1;
    let isRunning = false;
    while (!isRunning && attempts <= retries) {
      try {
        await rpcClient.ping();
        isRunning = true;
      } catch (e) {
        attempts += 1;
        if (attempts !== retries) {
          await wait(retryDelay * 1000);
        }
      }
    }
    return isRunning;
  }

  return isDashCoreRunning;
};
