const SimplifiedMNListDiff = require('@dashevo/dashcore-lib/lib/deterministicmnlist/SimplifiedMNListDiff');

const wait = require('../util/wait');
const LatestCoreChainLock = require('./LatestCoreChainLock');

const MissingChainLockError = require('./errors/MissingChainLockError');

/**
 * Check that core is synced (factory)
 *
 * @param {RpcClient} coreRpcClient
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @param {LatestCoreChainLock} latestCoreChainLock
 * @param {number} smlMaxListsLimit
 * @param {string} network
 * @param {BaseLogger} logger
 *
 * @returns {waitForCoreSync}
 */
function waitForSMLSyncFactory(
  coreRpcClient,
  latestCoreChainLock,
  simplifiedMasternodeList,
  smlMaxListsLimit,
  network,
  logger,
) {
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
   * @typedef waitForSMLSyncFactory
   *
   * @returns {Promise<void>}
   */
  async function waitForSMLSync() {
    // Wait for 1000 blocks height to make sure that DML is enabled
    let { result: currentBlock } = await coreRpcClient.getBlockCount();

    if (currentBlock < 1000) {
      do {
        await wait(10000);

        ({ result: currentBlock } = await coreRpcClient.getBlockCount());
      } while (currentBlock < 1000);
    }

    // ChainLock is required to get finalized SML that won't be reorged
    const chainLock = latestCoreChainLock.getChainLock();

    if (!chainLock) {
      throw new MissingChainLockError();
    }

    // Initialize SML with 16 diffs to have enough quorum information
    // to be able to verify signatures

    let latestRequestedHeight = 1;

    const startHeight = chainLock.height - smlMaxListsLimit;

    const { result: rawDiff } = await coreRpcClient.protx('diff', latestRequestedHeight, startHeight);

    const initialSmlDiffs = [
      new SimplifiedMNListDiff(rawDiff, network),
      ...await fetchDiffsPerBlock(startHeight, chainLock.height),
    ];

    simplifiedMasternodeList.applyDiffs(initialSmlDiffs);

    latestRequestedHeight = chainLock.height;

    logger.debug(`SML is initalized for heights ${startHeight} to ${chainLock.height}`);

    // Update SML on new chain locked block

    let isProcessing = false;
    latestCoreChainLock.on(LatestCoreChainLock.EVENTS.update, async (updatedChainLock) => {
      if (isProcessing) {
        return;
      }

      isProcessing = true;

      const smlDiffs = await fetchDiffsPerBlock(latestRequestedHeight, updatedChainLock.height);

      simplifiedMasternodeList.applyDiffs(smlDiffs);

      latestRequestedHeight = updatedChainLock.height;

      logger.debug(`SML is updated for heights ${latestRequestedHeight} to ${updatedChainLock.height}`);

      isProcessing = false;
    });
  }

  return waitForSMLSync;
}
module.exports = waitForSMLSyncFactory;
