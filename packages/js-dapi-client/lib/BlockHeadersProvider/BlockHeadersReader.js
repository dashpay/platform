const { EventEmitter } = require('events');

const EVENTS = {
  BLOCK_HEADERS: 'BLOCK_HEADERS',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  ERROR: 'error',
};

const COMMANDS = {
  HANDLE_FINISHED_STREAM: 'HANDLE_FINISHED_STREAM',
  HANDLE_STREAM_RETRY: 'HANDLE_STREAM_RETRY',
  HANDLE_STREAM_ERROR: 'HANDLE_STREAM_ERROR',
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

    /**
     * Holds references to the historical streams
     *
     * @type {*[]}
     */
    this.historicalStreams = [];
  }

  /**
   * Reads historical block heights using multiple streams
   *
   * @param {number} fromBlockHeight
   * @param {number} toBlockHeight
   * @returns {Promise<void>}
   */
  async readHistorical(fromBlockHeight, toBlockHeight) {
    if (this.historicalStreams.length) {
      throw new Error('Historical streams are already running');
    }

    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount === 0) {
      return;
    }

    if (totalAmount < 0) {
      throw new Error(`Invalid total amount of headers to read: ${totalAmount}`);
    }

    // Resubscribe to the stream in case of error, and replace the stream in the array
    this.on(COMMANDS.HANDLE_STREAM_RETRY, (oldStream, newStream) => {
      const index = this.historicalStreams.indexOf(oldStream);
      this.historicalStreams[index] = newStream;
    });

    // Remove stream from the array in case of error
    this.on(COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
      const index = this.historicalStreams.indexOf(stream);
      this.historicalStreams.splice(index, 1);
      this.emit(EVENTS.ERROR, e);
    });

    // Remove finished stream from the array and emit HISTORICAL_DATA_OBTAINED event
    this.on(COMMANDS.HANDLE_FINISHED_STREAM, (stream) => {
      const index = this.historicalStreams.indexOf(stream);
      this.historicalStreams.splice(index, 1);
      if (this.historicalStreams.length === 0) {
        this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      }
    });

    const numStreams = Math.min(
      Math.max(Math.round(totalAmount / this.targetBatchSize), 1),
      this.maxParallelStreams,
    );

    const actualBatchSize = Math.ceil(totalAmount / numStreams);
    for (let batchIndex = 0; batchIndex < numStreams; batchIndex += 1) {
      const startingHeight = (batchIndex * actualBatchSize) + 1;
      const count = Math.min(actualBatchSize, toBlockHeight - startingHeight + 1);

      const subscribeWithRetries = this.subscribeToHistoricalBatch(this.maxRetries);

      // eslint-disable-next-line no-await-in-loop
      const stream = await subscribeWithRetries(startingHeight, count);
      this.historicalStreams.push(stream);
    }
  }

  stopReadingHistorical() {
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_RETRY);
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_ERROR);
    this.removeAllListeners(COMMANDS.HANDLE_FINISHED_STREAM);
    this.historicalStreams.forEach((stream) => stream.destroy());
    this.historicalStreams = [];
  }

  /**
   * Subscribes to continuously arriving block headers
   *
   * @param {number} fromBlockHeight
   * @returns {Promise<Stream>}
   */
  async subscribeToNew(fromBlockHeight) {
    const stream = await this.coreMethods.subscribeToBlockHeadersWithChainLocks({
      fromBlockHeight,
    });

    stream.on('data', (data) => {
      const blockHeaders = data.getBlockHeaders();

      if (blockHeaders) {
        /**
         * Kills stream in case of deliberate rejection from the outside
         *
         * @param e
         */
        const rejectHeaders = (e) => {
          stream.destroy(e);
        };

        this.emit(EVENTS.BLOCK_HEADERS, blockHeaders.getHeadersList(), rejectHeaders);
      }
    });

    stream.on('error', (e) => {
      this.emit(EVENTS.ERROR, e);
    });

    return stream;
  }

  /**
   * A HOF that returns a function to subscribe to historical block headers and chain locks
   * and handles retry logic
   *
   * @private
   * @param {number} [maxRetries=0] - maximum amount of retries
   * @returns {function(*, *): Promise<Stream>}
   */
  subscribeToHistoricalBatch(maxRetries = 0) {
    let currentRetries = 0;

    /**
     * Subscribes to the stream of historical data and handles retry logic
     *
     * @param {number} fromBlockHeight
     * @param {number} count
     * @returns {Promise<Stream>}
     */
    const subscribeWithRetries = async (fromBlockHeight, count) => {
      let headersObtained = 0;

      const stream = await this.coreMethods.subscribeToBlockHeadersWithChainLocks({
        fromBlockHeight,
        count,
      });

      stream.on('data', (data) => {
        const blockHeaders = data.getBlockHeaders();

        if (blockHeaders) {
          const headersList = blockHeaders.getHeadersList();

          let rejected = false;

          /**
           * Kills stream in case of deliberate rejection from the outside
           *
           * @param e
           */
          const rejectHeaders = (e) => {
            rejected = true;
            stream.destroy(e);
          };

          this.emit(EVENTS.BLOCK_HEADERS, headersList, rejectHeaders);

          if (!rejected) {
            headersObtained += headersList.length;
          }
        }
      });

      stream.on('error', (streamError) => {
        if (currentRetries < maxRetries) {
          const newFromBlockHeight = fromBlockHeight + headersObtained;
          const newCount = count - headersObtained;

          subscribeWithRetries(newFromBlockHeight, newCount)
            .then((newStream) => {
              currentRetries += 1;
              this.emit(COMMANDS.HANDLE_STREAM_RETRY, stream, newStream);
            }).catch((e) => {
              this.emit(COMMANDS.HANDLE_STREAM_ERROR, stream, e);
            });
        } else {
          this.emit(COMMANDS.HANDLE_STREAM_ERROR, stream, streamError);
        }
      });

      stream.on('end', () => {
        this.emit(COMMANDS.HANDLE_FINISHED_STREAM, stream);
      });

      return stream;
    };

    return subscribeWithRetries;
  }
}

BlockHeadersReader.EVENTS = EVENTS;
BlockHeadersReader.COMMANDS = COMMANDS;

module.exports = BlockHeadersReader;
