const { BlockHeader } = require('@dashevo/dashcore-lib');

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
   * @param fromBlockHash
   * @param count
   * @return {AsyncIterableIterator<BlockHeader[]>}
   */
  async function* getHistoricalBlockHeadersIterator(
    fromBlockHash,
    count,
  ) {
    // TODO: implement `getblockstats` in dashd-rpc and use instead of getBlock
    const fromBlock = await coreRpcApi.getBlock(fromBlockHash);
    const fromHeight = fromBlock.height;
    const numberOfBatches = Math.ceil(count / MAX_HEADERS_PER_REQUEST);

    for (let batchIndex = 0; batchIndex < numberOfBatches; batchIndex++) {
      const currentHeight = fromHeight + batchIndex * MAX_HEADERS_PER_REQUEST;
      const blocksToScan = getBlocksToScan(batchIndex, numberOfBatches, count);

      const blockHash = await coreRpcApi.getBlockHash(currentHeight);

      // TODO: figure out whether it's possible to omit new BlockHeader() conversion
      // and directly send bytes to the client
      const blockHeaders = (await coreRpcApi.getBlockHeaders(
        blockHash, blocksToScan,
      )).map((rawBlockHeader) => new BlockHeader(Buffer.from(rawBlockHeader, 'hex')));

      yield blockHeaders;
    }
  }

  return getHistoricalBlockHeadersIterator;
}

module.exports = getHistoricalBlockHeadersIteratorFactory;
