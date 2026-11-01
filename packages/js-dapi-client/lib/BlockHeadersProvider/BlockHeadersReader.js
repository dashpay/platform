const { EventEmitter } = require('events');
const { BlockHeader } = require('@dashevo/dashcore-lib');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const EVENTS = {
  BLOCK_HEADERS: 'BLOCK_HEADERS',
  HISTORICAL_DATA_OBTAINED: 'HISTORICAL_DATA_OBTAINED',
  ERROR: 'error',
};

const CLIENT_ALREADY_CLOSED_ERROR_MESSAGE = 'Client already closed - cannot .close()';

/**
 * @typedef BlockHeadersReaderOptions
 * @property {number} [maxParallelStreams]
 * @property {number} [targetBatchSize]
 * @property {number} [maxRetries]
 * @property {function(): number} [getConnectedHeight] - height up to which
 *   headers joined the chain; bounds how far historical reads run ahead
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
    this.getConnectedHeight = options.getConnectedHeight;

    /**
     * Holds references to the running historical streams, in batch order
     * @type {Stream[]}
     */
    this.historicalStreams = [];

    /**
     * Batches of the current historical read; null when not reading
     * @type {{ fromBlockHeight: number, count: number, stream: ?Stream, done: boolean }[]|null}
     */
    this.historicalBatches = null;

    /**
     * Holds reference to the continuous sync stream
     * @type {Stream}
     */
    this.continuousSyncStream = null;
  }

  /**
   * Reads historical block headers in batches of about targetBatchSize, with
   * up to maxParallelStreams batches in flight. A batch starts only within
   * maxParallelStreams batches of the first one not yet settled: finished and,
   * with getConnectedHeight, joined to the chain. So headers waiting ahead of
   * the chain stay bounded.
   * @param {number} fromBlockHeight
   * @param {number} toBlockHeight
   * @returns {Promise<void>}
   */
  async readHistorical(fromBlockHeight, toBlockHeight) {
    if (this.historicalBatches) {
      throw new Error('Historical streams are already running. Please stop them first.');
    }

    if (fromBlockHeight <= 0) {
      throw new Error(`Invalid fromBlockHeight value: ${fromBlockHeight}`);
    }

    const totalAmount = toBlockHeight - fromBlockHeight + 1;
    if (totalAmount <= 0) {
      throw new Error(`Invalid total amount of headers to read: ${totalAmount}`);
    }

    const batchCount = Math.max(Math.round(totalAmount / this.targetBatchSize), 1);
    const batchSize = Math.ceil(totalAmount / batchCount);

    this.historicalBatches = [];
    for (let height = fromBlockHeight; height <= toBlockHeight; height += batchSize) {
      this.historicalBatches.push({
        fromBlockHeight: height,
        count: Math.min(batchSize, toBlockHeight - height + 1),
        stream: null,
        retry: null,
        done: false,
      });
    }
    this.nextBatchIndex = 0;
    this.firstUnsettledBatchIndex = 0;

    try {
      await this.startHistoricalBatches();
    } catch (e) {
      this.stopReadingHistorical();
      throw e;
    }
  }

  /**
   * @private
   * Starts the batches the window allows
   */
  async startHistoricalBatches() {
    const batches = this.historicalBatches;
    while (batches === this.historicalBatches
      && this.nextBatchIndex < batches.length
      && this.nextBatchIndex < this.firstUnsettledBatchIndex + this.maxParallelStreams) {
      const batch = batches[this.nextBatchIndex];
      this.nextBatchIndex += 1;

      const subscribeWithRetries = this.createSubscribeToHistoricalBatch(this.maxRetries, batch);
      // eslint-disable-next-line no-await-in-loop
      const stream = await subscribeWithRetries(batch.fromBlockHeight, batch.count);
      if (batches === this.historicalBatches) {
        batch.stream = stream;
        this.updateHistoricalStreams();
      } else {
        // Stopped while subscribing
        this.cancelStream(stream);
      }
    }
  }

  /** @private */
  updateHistoricalStreams() {
    this.historicalStreams = (this.historicalBatches || [])
      .filter(({ stream }) => stream)
      .map(({ stream }) => stream);
  }

  /**
   * @private
   * @param {object} batch
   * @param {Stream} stream
   */
  historicalBatchEnded(batch, stream) {
    if (!batch) {
      // A stream read on its own
      this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      return;
    }

    if (batch.stream !== stream || !this.historicalBatches) {
      // Ended after a retry replaced it or reading stopped
      return;
    }

    /* eslint-disable no-param-reassign */
    batch.stream = null;
    batch.done = true;
    /* eslint-enable no-param-reassign */
    this.updateHistoricalStreams();

    const batches = this.historicalBatches;
    if (batches.every(({ done }) => done)) {
      // Every batch arrived; did they all join the chain?
      const connectedHeight = this.getConnectedHeight ? this.getConnectedHeight() : Infinity;
      const unconnected = batches.find(({ fromBlockHeight, count }) => (
        connectedHeight < fromBlockHeight + count - 1));
      if (unconnected) {
        this.stopReadingHistorical();
        this.emit(EVENTS.ERROR, new Error(`Headers from height ${unconnected.fromBlockHeight} don't connect to the chain`));
        return;
      }
      this.historicalBatches = null;
      this.emit(EVENTS.HISTORICAL_DATA_OBTAINED);
      return;
    }

    this.advanceHistoricalWindow();
  }

  /**
   * Lets historical reads run further ahead once headers join the chain
   */
  chainAdvanced() {
    if (this.historicalBatches) {
      this.advanceHistoricalWindow();
    }
  }

  /** @private */
  advanceHistoricalWindow() {
    const batches = this.historicalBatches;
    const connectedHeight = this.getConnectedHeight ? this.getConnectedHeight() : Infinity;
    while (this.firstUnsettledBatchIndex < batches.length) {
      const { done, fromBlockHeight, count } = batches[this.firstUnsettledBatchIndex];
      if (!done || connectedHeight < fromBlockHeight + count - 1) {
        break;
      }
      this.firstUnsettledBatchIndex += 1;
    }

    const windowFull = this.nextBatchIndex >= this.firstUnsettledBatchIndex + this.maxParallelStreams;
    const inFlight = batches.slice(0, this.nextBatchIndex).some(({ done }) => !done);
    if (windowFull && !inFlight) {
      // Everything in the window arrived, but the first batch didn't connect
      const { fromBlockHeight } = batches[this.firstUnsettledBatchIndex];
      this.stopReadingHistorical();
      this.emit(EVENTS.ERROR, new Error(`Headers from height ${fromBlockHeight} don't connect to the chain`));
      return;
    }

    this.startHistoricalBatches().catch((e) => {
      if (batches !== this.historicalBatches) {
        // A later read replaced this one
        return;
      }
      this.stopReadingHistorical();
      this.emit(EVENTS.ERROR, e);
    });
  }

  stopReadingHistorical() {
    const streams = this.historicalStreams;
    // Detach the batches, so retries in flight cancel the streams they open
    (this.historicalBatches || []).forEach((batch) => {
      /* eslint-disable no-param-reassign */
      batch.stream = null;
      batch.retry = null;
      /* eslint-enable no-param-reassign */
    });
    this.historicalBatches = null;
    this.historicalStreams = [];
    streams.forEach((stream) => this.cancelStream(stream));
  }

  /**
   * Subscribes to continuously arriving block headers
   * @param {number} fromBlockHeight
   * @param {function(): number} [getResumeHeight] - height to resubscribe from
   *   after the stream drops or headers are rejected; by default the height of
   *   the last header received
   * @returns {Promise<ReconnectableStream>}
   */
  async subscribeToNew(fromBlockHeight, getResumeHeight) {
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
          // Cancel the transport only: the retry below needs the listeners, so
          // this cannot go through cancelStream, which unsubscribes from events
          this.cancelStreamTransport(stream);
          stream.retryOnError(e);
        };

        this.emit(EVENTS.BLOCK_HEADERS, {
          headers,
          headHeight: batchHeadHeight,
        }, rejectHeaders);
      }
    };

    const beforeReconnectHandler = (updateArguments) => {
      const resumeHeight = getResumeHeight ? getResumeHeight() : lastKnownChainHeight;
      updateArguments({
        fromBlockHeight: resumeHeight,
        count: 0,
      });

      lastKnownChainHeight = resumeHeight - 1;
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
   * @param {object} [batch] - the batch the stream reads, from readHistorical
   * @returns {function(*, *): Promise<Stream>}
   */
  createSubscribeToHistoricalBatch(maxRetries = 0, batch = null) {
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

      // Events of a stream its batch no longer reads from (it failed and a
      // retry replaced it, reading stopped, or it already delivered its batch)
      // are ignored. The reader detaches a stream before it cancels it, so a
      // CANCELLED error from a current stream came from the server and is
      // retried like any other.
      let finished = false;
      const isCurrent = () => !finished && (!batch || batch.stream === stream);

      const errorHandler = (streamError) => {
        if (!isCurrent()) {
          return;
        }

        if (headersObtained >= count) {
          // It already delivered its whole batch
          this.historicalBatchEnded(batch, stream);
          return;
        }

        if (!batch && streamError.code === GrpcErrorCodes.CANCELLED) {
          return;
        }

        if (currentRetries < maxRetries) {
          const newFromBlockHeight = fromBlockHeight + headersObtained;
          const newCount = count - headersObtained;
          currentRetries += 1;

          // Detach the failed stream at once, so its late events can't end
          // the batch while the retry connects
          const retry = {};
          if (batch) {
            /* eslint-disable no-param-reassign */
            batch.stream = null;
            batch.retry = retry;
            /* eslint-enable no-param-reassign */
            this.updateHistoricalStreams();
          }

          subscribeWithRetries(newFromBlockHeight, newCount)
            .then((newStream) => {
              if (!batch) {
                return;
              }
              if (batch.retry !== retry) {
                // Reading stopped in the meantime
                this.cancelStream(newStream);
                return;
              }
              /* eslint-disable no-param-reassign */
              batch.retry = null;
              batch.stream = newStream;
              /* eslint-enable no-param-reassign */
              this.updateHistoricalStreams();
            }).catch((e) => {
              if (batch && batch.retry !== retry) {
                return;
              }
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

        if (blockHeaders && isCurrent()) {
          const rawHeaders = blockHeaders.getHeadersList();
          if (headersObtained + rawHeaders.length > count) {
            // Never take more than was asked for: the rest would wait in memory
            errorHandler(new Error(`Stream sent more than the ${count} headers asked for`));
            this.cancelStream(stream);
            return;
          }

          const headersList = rawHeaders.map((header) => new BlockHeader(Buffer.from(header)));

          let rejected = false;

          /**
           * Kills stream in case of deliberate rejection from the outside
           * @param e
           */
          const rejectHeaders = (e) => {
            rejected = true;
            // Retry first, which detaches the stream, then cancel it
            errorHandler(e);
            this.cancelStream(stream);
          };

          const batchHeadHeight = fromBlockHeight + headersObtained;
          this.emit(EVENTS.BLOCK_HEADERS, {
            headers: headersList,
            headHeight: batchHeadHeight,
          }, rejectHeaders);

          if (!rejected) {
            headersObtained += headersList.length;

            // Done without waiting for the server to end the stream: close it
            // before releasing the batch's slot, so a server that never ends
            // its streams can't keep more than the window open
            if (headersObtained === count) {
              finished = true;
              this.cancelStream(stream);
              this.historicalBatchEnded(batch, stream);
            }
          }
        }
      };

      const endHandler = () => {
        if (!isCurrent()) {
          return;
        }
        if (headersObtained >= count) {
          this.historicalBatchEnded(batch, stream);
        } else {
          errorHandler(new Error(`Stream ended after ${headersObtained} of ${count} headers`));
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
  /**
   * Cancels the stream's transport, tolerating one the transport has already
   * closed. Listeners are left in place for callers that keep using the stream.
   * @param {ReconnectableStream} stream
   */
  // eslint-disable-next-line class-methods-use-this
  cancelStreamTransport(stream) {
    try {
      stream.cancel();
    } catch (e) {
      if (e.message !== CLIENT_ALREADY_CLOSED_ERROR_MESSAGE) {
        throw e;
      }
    }
  }

  cancelStream(stream) {
    stream.removeAllListeners();
    // The CANCELLED error a cancelled stream emits must not go unheard
    if (typeof stream.listenerCount === 'function' && stream.listenerCount('error') === 0) {
      stream.on('error', () => {});
    }
    this.cancelStreamTransport(stream);
  }
}

BlockHeadersReader.EVENTS = EVENTS;

module.exports = BlockHeadersReader;
