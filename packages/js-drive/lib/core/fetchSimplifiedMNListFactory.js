const { SimplifiedMNList } = require('@dashevo/dashcore-lib');

/**
 * @param {RpcClient} coreRpcClient
 * @returns {fetchSimplifiedMNList}
 */
function fetchSimplifiedMNListFactory(coreRpcClient) {
  /**
   * @typedef fetchSimplifiedMNList
   * @param {number} fromBlockHeight
   * @param {number} toBlockHeight
   * @returns {Promise<SimplifiedMNList>}
   */
  async function fetchSimplifiedMNList(fromBlockHeight, toBlockHeight) {
    const { result: rawDiff } = await coreRpcClient.protx('diff', fromBlockHeight, toBlockHeight, true);

    return new SimplifiedMNList(rawDiff);
  }

  return fetchSimplifiedMNList;
}

module.exports = fetchSimplifiedMNListFactory;
