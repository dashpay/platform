const { BlockHeader } = require('@dashevo/dashcore-lib');
const cache = require('./cache');

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
 * @return {getHistoricalBlockHeadersIterator}
 */
function getHistoricalBlockHeadersIteratorFactory(coreRpcApi) {
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

      let blocksToScan = getBlocksToScan(batchIndex, numberOfBatches, count);
      let blockHash = await coreRpcApi.getBlockHash(currentHeight);

      const blockHeights = [...Array(blocksToScan).keys()]
        .map((e, i) => currentHeight + i);

      const cachedBlockHeaders = blockHeights.map((height) => cache.get(height));
      const [firstCachedItem] = cachedBlockHeaders;

      let lastCachedIndex = -1;

      if (firstCachedItem) {
        const firstMissingIndex = cachedBlockHeaders.indexOf(undefined);

        // return cache if we do not miss anything
        if (cachedBlockHeaders.filter((e) => !!e).length === blocksToScan) {
          return cachedBlockHeaders;
        }

        if (firstMissingIndex !== -1) {
          lastCachedIndex = firstMissingIndex - 1;

          const rawBlockHeader = cachedBlockHeaders[lastCachedIndex];
          const blockHeader = BlockHeader.fromRawBlock(rawBlockHeader);

          blockHash = blockHeader.hash.toString('hex');
          blocksToScan -= lastCachedIndex;
        }
      }

      const missingBlockHeaders = await coreRpcApi.getBlockHeaders(blockHash, blocksToScan);
      const rawBlockHeaders = [...cachedBlockHeaders.slice(0,
        lastCachedIndex !== -1 ? lastCachedIndex : 0), ...missingBlockHeaders];

      missingBlockHeaders.forEach((e, i) => cache.set(currentHeight + i, e));

      // TODO: figure out whether it's possible to omit new BlockHeader() conversion
      // and directly send bytes to the client
      yield rawBlockHeaders.map((rawBlockHeader) => new BlockHeader(Buffer.from(rawBlockHeader, 'hex')));
    }
  }

  return getHistoricalBlockHeadersIterator;
}

module.exports = getHistoricalBlockHeadersIteratorFactory;
