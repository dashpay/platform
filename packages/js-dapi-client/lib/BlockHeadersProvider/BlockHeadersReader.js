const { EventEmitter } = require('events');
const { BlockHeader } = require('@dashevo/dashcore-lib');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const EVENTS = {
  BLOCK_HEADERS: 'BLOCK_HEADERS',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  ERROR: 'error',
};

/**
 * @typedef BlockHeadersReaderOptions
 * @property {number} [maxParallelStreams]
 * @property {number} [targetBatchSize]
 * @property {number} [maxRetries]
 */

class BlockHeadersReader extends EventEmitter {
  /**
   * @param {BlockHeadersReaderOptions} options
   * @param createHistoricalSyncStream
   * @param createContinuousSyncStream
   */
  // TODO move options to as last param
  // eslint-disable-next-line default-param-last
  constructor(options = {}, createHistoricalSyncStream, createContinuousSyncStream) {
    super();
    this.createHistoricalSyncStream = createHistoricalSyncStream;
    this.createContinuousSyncStream = createContinuousSyncStream;
    this.maxParallelStreams = options.maxParallelStreams;
    this.targetBatchSize = options.targetBatchSize;
    this.maxRetries = options.maxRetries;

    /**
     * Holds references to the historical streams
     * @type {Stream[]}
     */
    this.historicalStreams = [];

    /**
     * Holds reference to the continuous sync stream
     * @type {Stream}
     */
    this.continuousSyncStream = null;
  }

  /**
   * Reads historical block heights using multiple streams
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

    const numStreams = Math.min(
      Math.max(Math.round(totalAmount / this.targetBatchSize), 1),
      this.maxParallelStreams,
    );

    const actualBatchSize = Math.ceil(totalAmount / numStreams);

    for (let batchIndex = 0; batchIndex < numStreams; batchIndex += 1) {
      const startingHeight = (batchIndex * actualBatchSize) + fromBlockHeight;
      const count = Math.min(actualBatchSize, toBlockHeight - startingHeight + 1);

      const subscribeWithRetries = this.createSubscribeToHistoricalBatch(this.maxRetries);
      // eslint-disable-next-line no-await-in-loop
      const stream = await subscribeWithRetries(startingHeight, count);
      this.historicalStreams.push(stream);
    }
  }

  stopReadingHistorical() {
    while (this.historicalStreams.length) {
      const stream = this.historicalStreams.shift();
      this.cancelStream(stream);
    }
  }

  /**
   * Subscribes to continuously arriving block headers
   * @param {number} fromBlockHeight
   * @returns {Promise<ReconnectableStream>}
   */
  async subscribeToNew(fromBlockHeight) {
    if (this.continuousSyncStream) {
      throw new Error('Continuous sync has already been started');
    }

    if (typeof fromBlockHeight !== 'number' || fromBlockHeight < 1) {
      throw new Error(`Invalid fromBlockHeight: ${fromBlockHeight}`);
    }

    // We don't know yet whether we already have header at fromBlockHeight
    let lastKnownChainHeight = fromBlockHeight - 1;

    const stream = await this.createContinuousSyncStream(fromBlockHeight);

    const errorHandler = (e) => {
      this.continuousSyncStream = null;
      if (e.code === GrpcErrorCodes.CANCELLED) {
        return;
      }
      this.emit(EVENTS.ERROR, e);
    };

    const dataHandler = (data) => {
      const blockHeadersResponse = data.getBlockHeaders();

      if (blockHeadersResponse) {
        const rawHeaders = blockHeadersResponse.getHeadersList();

        const headers = rawHeaders.map((header) => new BlockHeader(Buffer.from(header)));

        lastKnownChainHeight += headers.length;
        const batchHeadHeight = lastKnownChainHeight - headers.length + 1;

        /**
         * Kills stream in case of deliberate rejection from the outside
         * @param e
         */
        const rejectHeaders = async (e) => {
          // Don't use cancelStream there because it's going to unsubscribe from events
          stream.cancel();
          stream.retryOnError(e);
        };

        this.emit(EVENTS.BLOCK_HEADERS, {
          headers,
          headHeight: batchHeadHeight,
        }, rejectHeaders);
      }
    };

    const beforeReconnectHandler = (updateArguments) => {
      updateArguments({
        fromBlockHeight: lastKnownChainHeight,
        count: 0,
      });

      lastKnownChainHeight -= 1;
    };

    const endHandler = () => {
      this.continuousSyncStream = null;
    };

    stream.on('data', dataHandler);
    stream.on('beforeReconnect', beforeReconnectHandler);
    stream.on('error', errorHandler);
    stream.on('end', endHandler);

    stream.removeAllListeners = () => {
      stream.removeListener('data', dataHandler);
      stream.removeListener('end', endHandler);
      stream.removeListener('beforeReconnect', beforeReconnectHandler);
    };

    this.continuousSyncStream = stream;
    return stream;
  }

  unsubscribeFromNew() {
    if (this.continuousSyncStream) {
      this.cancelStream(this.continuousSyncStream);
      this.continuousSyncStream = null;
    }
  }

  // TODO: refactor whole thing with ReconnectableStream that supports
  // retry on error logic
  /**
   * A HOF that returns a function to subscribe to historical block headers and chain locks
   * and handles retry logic
   * @private
   * @param {number} [maxRetries] - maximum amount of retries
   * @returns {function(*, *): Promise<Stream>}
   */
  createSubscribeToHistoricalBatch(maxRetries = 0) {
    let currentRetries = 0;

    /**
     * Subscribes to the stream of historical data and handles retry logic
     * @param {number} fromBlockHeight
     * @param {number} count
     * @returns {Promise<Stream>}
     */
    const subscribeWithRetries = async (fromBlockHeight, count) => {
      let headersObtained = 0;

      const stream = await this.createHistoricalSyncStream(fromBlockHeight, count);

      const errorHandler = (streamError) => {
        if (streamError.code === GrpcErrorCodes.CANCELLED) {
          // TODO: consider reworking with COMMANDS instead
          // of producing a side effect that alters class state
          const index = this.historicalStreams.indexOf(stream);
          if (index >= 0) {
            this.historicalStreams.splice(index, 1);
          }
          return;
        }

        if (currentRetries < maxRetries) {
          const newFromBlockHeight = fromBlockHeight + headersObtained;
          const newCount = count - headersObtained;

          // TODO: do not retry in case newCount is zero
          subscribeWithRetries(newFromBlockHeight, newCount)
            .then((newStream) => {
              const index = this.historicalStreams.indexOf(stream);
              this.historicalStreams[index] = newStream;
              currentRetries += 1;
            }).catch((e) => {
              this.stopReadingHistorical();
              this.emit(EVENTS.ERROR, e);
            });
        } else {
          this.stopReadingHistorical();
          this.emit(EVENTS.ERROR, streamError);
        }
      };

      const dataHandler = (data) => {
        const blockHeaders = data.getBlockHeaders();

        if (blockHeaders) {
          const headersList = blockHeaders.getHeadersList()
            .map((header) => new BlockHeader(Buffer.from(header)));

          let rejected = false;

          /**
           * Kills stream in case of deliberate rejection from the outside
           * @param e
           */
          const rejectHeaders = (e) => {
            rejected = true;
            // Cancel stream and unsubscribe from all data events
            // because they are going th be re-created in errorHandler
            this.cancelStream(stream);
            errorHandler(e);
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

      const endHandler = () => {
        const index = this.historicalStreams.indexOf(stream);
        this.historicalStreams.splice(index, 1);
        if (this.historicalStreams.length === 0) {
          this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
        }
      };

      // TODO: consider reworking with "pasued" mode to
      // control the data flow manually because SPV verification is
      // resource intense and backpressure could be a problem
      // also the whole thing severely affects UI responsiveness
      stream.on('data', dataHandler);
      stream.on('error', errorHandler);
      stream.on('end', endHandler);

      stream.removeAllListeners = () => {
        stream.removeListener('data', dataHandler);
        stream.removeListener('end', endHandler);
      };

      return stream;
    };

    return subscribeWithRetries;
  }

  // eslint-disable-next-line class-methods-use-this
  cancelStream(stream) {
    stream.removeAllListeners();
    stream.cancel();
  }
}

BlockHeadersReader.EVENTS = EVENTS;

module.exports = BlockHeadersReader;
