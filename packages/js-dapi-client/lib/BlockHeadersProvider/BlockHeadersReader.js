const { EventEmitter } = require('events');

const EVENTS = {
  BLOCK_HEADERS: 'BLOCK_HEADERS',
  HISTORICAL_BLOCK_HEADERS_OBTAINED: 'HISTORICAL_BLOCK_HEADERS_OBTAINED',
};

/**
 * @typedef {BlockHeadersReaderOptions} BlockHeadersReaderOptions
 * @property {CoreMethodsFacade} [coreMethods]
 * @property {number} [maxParallelStreams]
 * @property {number} [targetBatchSize]
 */

class BlockHeadersReader extends EventEmitter {
  /**
   * @param {BlockHeadersReaderOptions} options
   */
  constructor(options = {}) {
    super();
    this.coreMethods = options.coreMethods;
    this.maxParallelStreams = options.maxParallelStreams;
    this.targetBatchSize = options.targetBatchSize;
  }

  async readHistorical(fromBlockHeight, totalCount) {
    const totalAmount = totalCount - fromBlockHeight;
    if (totalAmount < 1) {
      throw new Error(`Invalid total amount of headers to read: ${totalAmount}`);
    }

    const numStreams = Math.min(
      Math.max(Math.round(totalAmount / this.targetBatchSize), 1),
      this.maxParallelStreams,
    );

    let finishedStreams = 0;
    const onStreamEnded = () => {
      finishedStreams += 1;

      if (finishedStreams === numStreams) {
        this.emit(EVENTS.HISTORICAL_BLOCK_HEADERS_OBTAINED);
      }
    };

    const actualBatchSize = Math.ceil(totalAmount / numStreams);

    for (let batchIndex = 0; batchIndex < numStreams; batchIndex += 1) {
      const startingHeight = (batchIndex * actualBatchSize) + 1;
      const count = Math.min(actualBatchSize, totalCount - startingHeight + 1);

      // eslint-disable-next-line no-await-in-loop
      const stream = await this.coreMethods.subscribeToBlockHeadersWithChainLocks({
        fromBlockHeight: startingHeight,
        count,
      });

      stream.on('data', (data) => {
        const blockHeaders = data.getBlockHeaders();

        if (blockHeaders) {
          this.emit(EVENTS.BLOCK_HEADERS, blockHeaders.getHeadersList());
        }
      });

      stream.on('error', (e) => {
        throw e;
      });

      stream.on('end', onStreamEnded);
    }
  }

  async subscribeToNew(fromBlockHeight) {
    const stream = await this.coreMethods.subscribeToBlockHeadersWithChainLocks({
      fromBlockHeight,
    });

    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        this.emit(EVENTS.BLOCK_HEADERS, blockHeaders.getHeadersList());
      }
    });

    stream.on('error', (e) => {
      throw e;
    });
  }
}

BlockHeadersReader.EVENTS = EVENTS;

module.exports = BlockHeadersReader;
