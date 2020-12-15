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
      const { result: rawDiff } = await coreRpcClient.protx('diff', height, height + 1);

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
   *
   * @returns {Promise<void>}
   */
  async function updateSimplifiedMasternodeList(coreHeight) {
    // Should be enough to get 16 diffs
    if (coreHeight < smlMaxListsLimit + 1) {
      throw new NotEnoughBlocksForValidSMLError(coreHeight);
    }

    if (latestRequestedHeight === 1) {
      // Initialize SML with 16 diffs to have enough quorum information
      // to be able to verify signatures

      const startHeight = coreHeight - smlMaxListsLimit;

      const { result: rawDiff } = await coreRpcClient.protx('diff', latestRequestedHeight, startHeight);

      const initialSmlDiffs = [
        new SimplifiedMNListDiff(rawDiff, network),
        ...await fetchDiffsPerBlock(startHeight, coreHeight),
      ];

      simplifiedMasternodeList.applyDiffs(initialSmlDiffs);

      latestRequestedHeight = coreHeight;

      logger.debug(`SML is initialized for heights ${startHeight} to ${coreHeight}`);
    } else if (latestRequestedHeight < coreHeight) {
      // Update SML

      // We need only last 16 blocks for signature verification
      if (coreHeight - latestRequestedHeight > smlMaxListsLimit) {
        latestRequestedHeight = coreHeight - smlMaxListsLimit;
      }

      const smlDiffs = await fetchDiffsPerBlock(latestRequestedHeight, coreHeight);

      simplifiedMasternodeList.applyDiffs(smlDiffs);

      logger.debug(`SML is updated for heights ${latestRequestedHeight} to ${coreHeight}`);

      latestRequestedHeight = coreHeight;
    }
  }

  return updateSimplifiedMasternodeList;
}

module.exports = updateSimplifiedMasternodeListFactory;
