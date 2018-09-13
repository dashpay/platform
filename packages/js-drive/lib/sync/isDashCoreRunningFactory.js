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
  async function isDashCoreRunning(retries = 0, retryDelay = 5) {
    const tries = retries + 1;
    let attempts = 0;
    let isRunning = false;
    while (!isRunning && attempts < tries) {
      try {
        await rpcClient.ping();
        isRunning = true;
      } catch (e) {
        attempts += 1;
        if (attempts !== tries) {
          await wait(retryDelay * 1000);
        }
      }
    }
    return isRunning;
  }

  return isDashCoreRunning;
};
