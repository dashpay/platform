const { EventEmitter } = require('events');
const { BlockHeader } = require('@dashevo/dashcore-lib');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const DAPIStream = require('../transport/DAPIStream');

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

    /**
     * Holds reference to the continuous sync stream
     *
     * @type {DAPIStream}
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

    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount <= 0) {
      throw new Error(`Invalid total amount of headers to read: ${totalAmount}`);
    }

    // Resubscribe to the stream in case of error, and replace the stream in the array
    this.on(COMMANDS.HANDLE_STREAM_RETRY, (oldStream, newStream) => {
      const index = this.historicalStreams.indexOf(oldStream);
      this.historicalStreams[index] = newStream;
    });

    this.on(COMMANDS.HANDLE_STREAM_CANCELLATION, (stream) => {
      const index = this.historicalStreams.indexOf(stream);
      this.historicalStreams.splice(index, 1);
    });

    // Remove stream from the array in case of error
    this.on(COMMANDS.HANDLE_STREAM_ERROR, (stream, e) => {
      const index = this.historicalStreams.indexOf(stream);
      this.historicalStreams.splice(index, 1);
      this.emit(EVENTS.ERROR, e);
    });

    // Remove finished stream from the array and emit HISTORICAL_DATA_OBTAINED event
    this.on(COMMANDS.HANDLE_STREAM_END, (stream) => {
      const index = this.historicalStreams.indexOf(stream);
      this.historicalStreams.splice(index, 1);
      if (this.historicalStreams.length === 0) {
        this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      }
    });

    // TODO: consider reworking with minBatchSize instead of targetBatchSize
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
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_RETRY);
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_ERROR);
    this.removeAllListeners(COMMANDS.HANDLE_STREAM_END);
    this.historicalStreams.forEach((stream) => stream.cancel());
    this.historicalStreams = [];
  }

  stopContinuousSync() {
    if (this.continuousSyncStream) {
      this.continuousSyncStream.cancel();
      this.continuousSyncStream = null;
      // throw new Error('Continuous sync has not been started');
    }
  }

  // TODO: write tests
  // - it should correctly maintain lastKnownChainHeight after reconnects
  /**
   * Subscribes to continuously arriving block headers
   *
   * @param {number} fromBlockHeight
   * @returns {Promise<DAPIStream>}
   */
  async subscribeToNew(fromBlockHeight) {
    let lastKnownChainHeight = fromBlockHeight - 1;

    const stream = await DAPIStream
      .create(
        this.coreMethods.subscribeToBlockHeadersWithChainLocks,
        { reconnectTimeoutDelay: 5000 }, // TODO: remove after testing is done
      )({
        fromBlockHeight,
      });

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
        this.emit(EVENTS.BLOCK_HEADERS, headers, batchHeadHeight, rejectHeaders);
      }
    });

    stream.on('beforeReconnect', (updateArguments) => {
      updateArguments({
        fromBlockHeight: lastKnownChainHeight,
      });

      lastKnownChainHeight -= 1;
    });

    stream.on('error', (e) => {
      if (e.code === GrpcErrorCodes.CANCELLED) {
        return;
      }

      this.emit(EVENTS.ERROR, e);
    });
    this.continuousSyncStream = stream;
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
              this.emit(COMMANDS.HANDLE_STREAM_ERROR, stream, e);
            });
        } else {
          this.emit(COMMANDS.HANDLE_STREAM_ERROR, stream, streamError);
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
}

BlockHeadersReader.EVENTS = EVENTS;
BlockHeadersReader.COMMANDS = COMMANDS;

module.exports = BlockHeadersReader;
