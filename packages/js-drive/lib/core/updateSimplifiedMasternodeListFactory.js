const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');

const NotEnoughBlocksForValidSMLError = require('./errors/NotEnoughBlocksForValidSMLError');

/**
 * Check that core is synced (factory)
 *
 * @param {RpcClient} coreRpcClient
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {number} smlMaxListsLimit
 * @param {string} network
 * @param {BaseLogger} logger
 *
 * @returns {updateSimplifiedMasternodeList}
 */
function updateSimplifiedMasternodeListFactory(
  coreRpcClient,
  simplifiedMasternodeList,
  smlMaxListsLimit,
  network,
  logger,
) {
  // 1 means first block
  let latestRequestedHeight = 1;

  /**
   * @param {number} fromHeight
   * @param {number} toHeight
   * @return {Promise<SimplifiedMNListDiff[]>}
   */
  async function fetchDiffsPerBlock(fromHeight, toHeight) {
    const diffs = [];

    for (let height = fromHeight; height < toHeight; height += 1) {
      const { result: rawDiff } = await coreRpcClient.protx('diff', height, height + 1, true);

      const diff = new SimplifiedMNListDiff(rawDiff, network);

      diffs.push(diff);
    }

    return diffs;
  }

  /**
   * Check that core is synced
   *
   * @typedef updateSimplifiedMasternodeList
   * @param {number} coreHeight
   * @param {Object} [options]
   * @param {BaseLogger} [options.logger]
   *
   * @returns {Promise<boolean>}
   */
  async function updateSimplifiedMasternodeList(coreHeight, options = {}) {
    // either use a logger passed or use standard logger
    const contextLogger = (options.logger || logger);

    // Should be enough to get 16 diffs
    if (coreHeight < smlMaxListsLimit + 1) {
      throw new NotEnoughBlocksForValidSMLError(coreHeight);
    }

    // When we got more than 16 blocks of difference between last requested height
    // and core height, we take only last 16 of them.
    if (coreHeight - latestRequestedHeight > smlMaxListsLimit) {
      latestRequestedHeight = 1;
      simplifiedMasternodeList.reset();
    }

    if (latestRequestedHeight === 1) {
      // Initialize SML with 16 diffs to have enough quorum information
      // to be able to verify signatures

      const startHeight = coreHeight - smlMaxListsLimit;

      const { result: rawDiff } = await coreRpcClient.protx('diff', latestRequestedHeight, startHeight, true);

      const initialSmlDiffs = [
        new SimplifiedMNListDiff(rawDiff, network),
        ...await fetchDiffsPerBlock(startHeight, coreHeight),
      ];

      simplifiedMasternodeList.applyDiffs(initialSmlDiffs);

      latestRequestedHeight = coreHeight;

      contextLogger.debug(`SML is initialized for core heights ${startHeight} to ${coreHeight}`);

      return true;
    }

    if (latestRequestedHeight < coreHeight) {
      // Update SML

      const smlDiffs = await fetchDiffsPerBlock(latestRequestedHeight, coreHeight);

      simplifiedMasternodeList.applyDiffs(smlDiffs);

      contextLogger.debug(`SML is updated for core heights ${latestRequestedHeight} to ${coreHeight}`);

      latestRequestedHeight = coreHeight;

      return true;
    }

    return false;
  }

  return updateSimplifiedMasternodeList;
}

module.exports = updateSimplifiedMasternodeListFactory;
