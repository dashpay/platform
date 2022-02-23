const MAX_HEADERS_PER_REQUEST = 500;

/**
 * @param {number} batchIndex
 * @param {number} numberOfBatches
 * @param {number} totalCount
 * @return {number}
 */
function getBlocksToScan(batchIndex, numberOfBatches, totalCount) {
  const isLastBatch = batchIndex + 1 === numberOfBatches;
  return isLastBatch
    ? totalCount - batchIndex * MAX_HEADERS_PER_REQUEST
    : MAX_HEADERS_PER_REQUEST;
}

/**
 * @param {CoreRpcClient} coreRpcApi
 * @param {ChainDataProvider} chainDataProvider
 * @return {getHistoricalBlockHeadersIterator}
 */
function getHistoricalBlockHeadersIteratorFactory(coreRpcApi, chainDataProvider) {
  /**
   * @typedef getHistoricalBlockHeadersIterator
   * @param fromBlockHeight {number}
   * @param count {number}
   * @return {AsyncIterableIterator<BlockHeader[]>}
   */
  async function* getHistoricalBlockHeadersIterator(
    fromBlockHeight,
    count,
  ) {
    const numberOfBatches = Math.ceil(count / MAX_HEADERS_PER_REQUEST);

    for (let batchIndex = 0; batchIndex < numberOfBatches; batchIndex++) {
      const currentHeight = fromBlockHeight + batchIndex * MAX_HEADERS_PER_REQUEST;

      const blocksToScan = getBlocksToScan(batchIndex, numberOfBatches, count);

      const blockHash = await coreRpcApi.getBlockHash(currentHeight);

      const blockHeaders = await chainDataProvider.getBlockHeaders(blockHash, currentHeight, blocksToScan);

      yield blockHeaders;
    }
  }

  return getHistoricalBlockHeadersIterator;
}

module.exports = getHistoricalBlockHeadersIteratorFactory;
