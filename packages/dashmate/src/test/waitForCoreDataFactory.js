import wait from '../util/wait.js';

/**
 * @param {CoreRpcClient} rpcClient
 * @returns {waitForCoreData}
 */

export default function waitForCoreDataFactory(rpcClient) {
  /**
   * @typedef {function} waitForCoreData
   * @param {number} originalValue
   * @param {function(number, number)} predicateFn
   * @returns {Promise<number>}
   */
  async function waitForCoreData(originalValue, predicateFn) {
    let result = originalValue;

    while (!predicateFn(result, originalValue)) {
      await wait(10000); // 10 seconds

      const blockchainInfo = await rpcClient.getBlockchainInfo();

      result = blockchainInfo.result.headers;
    }

    return result;
  }

  return waitForCoreData;
}
