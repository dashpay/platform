const { EventEmitter } = require('events');
const { BlockHeader } = require('@dashevo/dashcore-lib');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const EVENTS = {
  BLOCK_HEADERS: 'BLOCK_HEADERS',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  ERROR: 'error',
};

const COMMANDS = {
  HANDLE_STREAM_END: 'HANDLE_STREAM_END',
  HANDLE_STREAM_RETRY: 'HANDLE_STREAM_RETRY',
  HANDLE_STREAM_ERROR: 'HANDLE_STREAM_ERROR',
  HANDLE_STREAM_CANCELLATION: 'HANDLE_STREAM_CANCELLATION',
};

/**
 * @typedef BlockHeadersReaderOptions
 * @property {CoreMethodsFacade} [coreMethods]
 * @property {Function} [createHistoricalSyncStream]
 * @property {Function} [createContinuousSyncStream]
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
    this.createHistoricalSyncStream = options.createHistoricalSyncStream;
    this.createContinuousSyncStream = options.createContinuousSyncStream;
    this.maxParallelStreams = options.maxParallelStreams;
    this.targetBatchSize = options.targetBatchSize;
    this.maxRetries = options.maxRetries;

    /**
     * Holds references to the historical streams
     *
     * @type {Stream[]}
     */
    this.historicalStreams = [];

    /**
     * Holds reference to the continuous sync stream
     *
     * @type {Stream}
     */
    this.continuousSyncStream = null;
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
      throw new Error('Historical streams are already running. Please stop them first.');
    }

    if (fromBlockHeight <= 0) {
      throw new Error(`Invalid fromBlockHeight value: ${fromBlockHeight}`);
    }

    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount <= 0) {
      throw new Error(`Invalid total amount of headers to read: ${totalAmount}`);
    }

    // Resubscribe to the stream in case of error, and replace the stream in the array
    this.on(COMMANDS.HANDLE_STREAM_RETRY, (oldStream, newStream) => {
      const index = this.historicalStreams.indexOf(oldStream);
      this.removeStreamListeners(oldStream);
      this.historicalStreams[index] = newStream;
    });

    this.on(COMMANDS.HANDLE_STREAM_CANCELLATION, (stream) => {
      const index = this.historicalStreams.indexOf(stream);
      this.removeStreamListeners(stream);
      this.historicalStreams.splice(index, 1);

      if (this.historicalStreams.length === 0) {
        this.removeCommandListeners();
      }
    });

    this.on(COMMANDS.HANDLE_STREAM_ERROR, (e) => {
      this.stopReadingHistorical();
      this.emit(EVENTS.ERROR, e);
    });

    // Remove finished stream from the array and emit HISTORICAL_DATA_OBTAINED event
    this.on(COMMANDS.HANDLE_STREAM_END, (stream) => {
      this.removeStreamListeners(stream);
      const index = this.historicalStreams.indexOf(stream);
      this.historicalStreams.splice(index, 1);
      if (this.historicalStreams.length === 0) {
        this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
        this.removeCommandListeners();
      }
    });

    const numStreams = Math.min(
      Math.max(Math.round(totalAmount / this.targetBatchSize), 1),
      this.maxParallelStreams,
    );

    const actualBatchSize = Math.ceil(totalAmount / numStreams);

    for (let batchIndex = 0; batchIndex < numStreams; batchIndex += 1) {
      const startingHeight = (batchIndex * actualBatchSize) + fromBlockHeight;
      const count = Math.min(actualBatchSize, toBlockHeight - startingHeight + 1);

      const subscribeWithRetries = this.subscribeToHistoricalBatch(this.maxRetries);
      // eslint-disable-next-line no-await-in-loop
      const stream = await subscribeWithRetries(startingHeight, count);
      this.historicalStreams.push(stream);
    }
  }

  stopReadingHistorical() {
    this.removeCommandListeners();
    this.historicalStreams.forEach((stream) => {
      stream.cancel();
      this.removeStreamListeners(stream);
    });
    this.historicalStreams = [];
  }

  /**
   * @private
   */
  removeCommandListeners() {
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_RETRY);
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_ERROR);
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_END);
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_CANCELLATION);
  }

  /**
   * Subscribes to continuously arriving block headers
   *
   * @param {number} fromBlockHeight
   * @returns {Promise<DAPIStream>}
   */
  async subscribeToNew(fromBlockHeight) {
    if (this.continuousSyncStream) {
      throw new Error('Continuous sync has already been started');
    }

    if (fromBlockHeight < 1) {
      throw new Error(`Invalid fromBlockHeight: ${fromBlockHeight}`);
    }

    let lastKnownChainHeight = fromBlockHeight - 1;

    const stream = await this.createContinuousSyncStream(fromBlockHeight);

    stream.on('data', (data) => {
      const blockHeadersResponse = data.getBlockHeaders();

      if (blockHeadersResponse) {
        const rawHeaders = blockHeadersResponse.getHeadersList();

        const headers = rawHeaders.map((header) => new BlockHeader(Buffer.from(header)));

        lastKnownChainHeight += headers.length;
        const batchHeadHeight = lastKnownChainHeight - headers.length + 1;

        /**
         * Kills stream in case of deliberate rejection from the outside
         *
         * @param e
         */
        const rejectHeaders = (e) => {
          stream.destroy(e);
        };
        this.emit(EVENTS.BLOCK_HEADERS, {
          headers,
          headHeight: batchHeadHeight,
        }, rejectHeaders);
      }
    });

    stream.on('beforeReconnect', (updateArguments) => {
      updateArguments({
        fromBlockHeight: lastKnownChainHeight,
      });

      lastKnownChainHeight -= 1;
    });

    stream.on('error', (e) => {
      this.continuousSyncStream = null;
      if (e.code === GrpcErrorCodes.CANCELLED) {
        return;
      }
      this.emit(EVENTS.ERROR, e);
    });

    stream.on('end', () => {
      this.continuousSyncStream = null;
    });

    this.continuousSyncStream = stream;
    return stream;
  }

  unsubscribeFromNew() {
    if (this.continuousSyncStream) {
      // Get a reference before cancellation because it will be nulled
      const stream = this.continuousSyncStream;
      stream.cancel();
      this.removeStreamListeners(stream);
      stream.removeAllListeners('beforeReconnect');
      this.continuousSyncStream = null;
    }
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

      const stream = await this.createHistoricalSyncStream(fromBlockHeight, count);

      const dataHandler = (data) => {
        const blockHeaders = data.getBlockHeaders();

        if (blockHeaders) {
          const headersList = blockHeaders.getHeadersList()
            .map((header) => new BlockHeader(Buffer.from(header)));

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

          const batchHeadHeight = fromBlockHeight + headersObtained;
          this.emit(EVENTS.BLOCK_HEADERS, {
            headers: headersList,
            headHeight: batchHeadHeight,
          }, rejectHeaders);

          if (!rejected) {
            headersObtained += headersList.length;
          }
        }
      };

      const errorHandler = (streamError) => {
        if (streamError.code === GrpcErrorCodes.CANCELLED) {
          this.emit(COMMANDS.HANDLE_STREAM_CANCELLATION, stream);
          return;
        }

        if (currentRetries < maxRetries) {
          const newFromBlockHeight = fromBlockHeight + headersObtained;
          const newCount = count - headersObtained;

          subscribeWithRetries(newFromBlockHeight, newCount)
            .then((newStream) => {
              currentRetries += 1;
              this.emit(COMMANDS.HANDLE_STREAM_RETRY, stream, newStream);
            }).catch((e) => {
              this.emit(COMMANDS.HANDLE_STREAM_ERROR, e);
            });
        } else {
          this.emit(COMMANDS.HANDLE_STREAM_ERROR, streamError);
        }
      };

      const endHandler = () => {
        this.emit(COMMANDS.HANDLE_STREAM_END, stream);
      };

      stream.on('data', dataHandler);
      stream.on('error', errorHandler);
      stream.on('end', endHandler);

      return stream;
    };

    return subscribeWithRetries;
  }

  /**
   * @private
   * @param stream
   */
  removeStreamListeners(stream) {
    stream.removeAllListeners('data');
    stream.removeAllListeners('error');
    stream.removeAllListeners('end');
  }
}

BlockHeadersReader.EVENTS = EVENTS;
BlockHeadersReader.COMMANDS = COMMANDS;

module.exports = BlockHeadersReader;
