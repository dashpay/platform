const {BlockHeader} = require('@dashevo/dashcore-lib');
const cache = require('../core/cache')

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
        .map((e, i) => {
          return currentHeight + i + 1
        })

      const cachedBlockHeaders = blockHeights.map((height) => cache.get(height))
      const lastCachedIndex = (cachedBlockHeaders.findIndex((e) => e === undefined)) - 1

      if (lastCachedIndex >= 0) {
        const rawBlockHeader = cachedBlockHeaders[lastCachedIndex]
        const blockHeader = BlockHeader.fromRawBlock(rawBlockHeader)

        blockHash = blockHeader.getHash()
        blocksToScan = (blocksToScan - lastCachedIndex) - 1

        if (blocksToScan === 0) {
          return cachedBlockHeaders
        }
      }

      const rawBlockHeaders = await coreRpcApi.getBlockHeaders(blockHash, blocksToScan)

      rawBlockHeaders.forEach((e, i) => cache.set(currentHeight + i, e))

      // TODO: figure out whether it's possible to omit new BlockHeader() conversion
      // and directly send bytes to the client
      yield rawBlockHeaders.map((rawBlockHeader) => new BlockHeader(Buffer.from(rawBlockHeader, 'hex')));
    }
  }

  return getHistoricalBlockHeadersIterator;
}

module.exports = getHistoricalBlockHeadersIteratorFactory;
