const { EventEmitter } = require('events');

const EVENTS = {
  BLOCK_HEADERS: 'BLOCK_HEADERS',
  ERROR: 'ERROR',
};

/**
 * @typedef BlockHeadersReaderOptions
 * @property {CoreMethodsFacade} [coreMethods]
 * @property {number} [maxParallelStreams]
 * @property {number} [targetBatchSize]
 * @property {number} [maxRetries]
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
    this.maxRetries = options.maxRetries;
  }

  /**
   * Reads historical block heights using multiple streams
   *
   * @param fromBlockHeight
   * @param toBlockHeight
   * @returns {Promise<void>}
   */
  async readHistorical(fromBlockHeight, toBlockHeight) {
    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount === 0) {
      return;
    }

    if (totalAmount < 0) {
      throw new Error(`Invalid total amount of headers to read: ${totalAmount}`);
    }

    const numStreams = Math.min(
      Math.max(Math.round(totalAmount / this.targetBatchSize), 1),
      this.maxParallelStreams,
    );

    const actualBatchSize = Math.ceil(totalAmount / numStreams);

    const streamsPromises = Array.from({ length: numStreams })
      .map((_, batchIndex) => {
        const startingHeight = (batchIndex * actualBatchSize) + 1;
        const count = Math.min(actualBatchSize, toBlockHeight - startingHeight + 1);

        const subscribe = this.createBatchFetcher();

        return subscribe(startingHeight, count);
      });

    await Promise.all(streamsPromises);
  }

  /**
   * Subscribes to continuously arriving block headers
   *
   * @param fromBlockHeight
   * @returns {Promise<Stream>}
   */
  async subscribeToNew(fromBlockHeight) {
    const stream = await this.coreMethods.subscribeToBlockHeadersWithChainLocks({
      fromBlockHeight,
    });

    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        try {
          this.emit(EVENTS.BLOCK_HEADERS, blockHeaders.getHeadersList(), (e) => {
            stream.destroy(e);
          });
        } catch (e) {
          this.emit(EVENTS.ERROR, e);
        }
      }
    });

    stream.on('error', (e) => {
      this.emit(EVENTS.ERROR, e);
    });

    return stream;
  }

  /**
   * A HOC that returns a function to subscribe to block headers and chain locks
   * and handles retry logic
   *
   * @private
   * @returns {function(*, *): Promise<Stream>}
   */
  createBatchFetcher() {
    let currentRetries = 0;

    /**
     * Subscribes to the stream of historical data and handles retry logic
     *
     * @param fromBlockHeight
     * @param count
     * @returns {Promise<Stream>}
     */
    const fetchBatch = async (fromBlockHeight, count) => new Promise((resolve, reject) => {
      let headersObtained = 0;

      this.coreMethods.subscribeToBlockHeadersWithChainLocks({
        fromBlockHeight,
        count,
      }).then((stream) => {
        stream.on('data', (data) => {
          const blockHeaders = data.getBlockHeaders();

          if (blockHeaders) {
            const headersList = blockHeaders.getHeadersList();

            let rejected = false;
            try {
              this.emit(EVENTS.BLOCK_HEADERS, headersList, (e) => {
                rejected = true;
                stream.destroy(e);
              });
            } catch (e) {
              this.emit(EVENTS.ERROR, e);
            }

            if (!rejected) {
              headersObtained += headersList.length;
            }
          }
        });

        stream.on('error', (e) => {
          if (currentRetries < this.maxRetries) {
            const newFromBlockHeight = fromBlockHeight + headersObtained;
            const newCount = count - headersObtained;
            fetchBatch(newFromBlockHeight, newCount)
              .then(resolve)
              .catch(reject);
          } else {
            reject(e);
          }

          currentRetries += 1;
        });

        stream.on('end', () => {
          resolve();
        });
      });
    });

    return fetchBatch;
  }
}

BlockHeadersReader.EVENTS = EVENTS;

module.exports = BlockHeadersReader;
