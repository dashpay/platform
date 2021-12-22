const { BlockHeader } = require('@dashevo/dashcore-lib');

const log = require('../log');

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
    const fromBlock = await coreRpcApi.getBlock(fromBlockHash);
    const fromHeight = fromBlock.height;
    const numberOfBatches = Math.ceil(count / MAX_HEADERS_PER_REQUEST);
    log.info(`getHistoricalBlockHeaders(): ${count}, ${fromHeight}`);

    for (let batchIndex = 0; batchIndex < numberOfBatches; batchIndex++) {
      const currentHeight = fromHeight + batchIndex * MAX_HEADERS_PER_REQUEST;
      const blocksToScan = getBlocksToScan(batchIndex, numberOfBatches, count);

      const blockHash = await coreRpcApi.getBlockHash(currentHeight);

      log.info(`Iter(${batchIndex}): ${blockHash}, ${blocksToScan}`);

      let blockHeaders = (await coreRpcApi.getBlockHeaders(
        blockHash, blocksToScan,
      ));

      // TODO: figure out whether it's possible to omit BlockHeader.fromBuffer conversion
      // and directly send bytes to the client
      blockHeaders = blockHeaders.map((blockHeader) => BlockHeader.fromBuffer(blockHeader));

      yield blockHeaders;
    }
  }

  return getHistoricalBlockHeadersIterator;
}

module.exports = getHistoricalBlockHeadersIteratorFactory;
