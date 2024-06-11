const EventEmitter = require('events');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');
const wait = require('../utils/wait');

/**
 * @typedef ReconnectableStreamOptions
 * @property {number} [autoReconnectInterval]
 *    interval in MS to perform auto reconnect
 * @property {number} [maxRetriesOnError]
 *    maximum amount of retry attempts on error happens. If set to -1, retries are infinite
 * @property {number} [retryOnErrorDelay]
 *    delay in MS to perform retry after an error
 */
const defaultOptions = {
  autoReconnectInterval: 600000,
  maxRetriesOnError: 10,
  retryOnErrorDelay: 1000,
};

const EVENTS = {
  DATA: 'data',
  ERROR: 'error',
  END: 'end',
  BEFORE_RECONNECT: 'beforeReconnect',
};

/**
 * A wrapper around grpc-js/grpc-web streams that provides auto-reconnect
 * and retry on error functionality
 * - Auto reconnect is needed to not hang on one node for too long.
 *   It happens within provided interval
 * - Retry on error logic performs retry attempts until it reaches it's limit.
 *   After reaching limit, ReconnectableStream emits an error.
 */
class ReconnectableStream extends EventEmitter {
  /**
   * Helper that wraps stream creation function and performs auto connection
   * @param {Function} createStreamFunction - function returning grpc-js/grpc-web stream
   * @param {ReconnectableStreamOptions} options
   * @returns {function(...[*]): Promise<ReconnectableStream>}
   */
  static create(createStreamFunction, options) {
    return async (...args) => {
      const reconnectableStream = new ReconnectableStream(createStreamFunction, options);
      await reconnectableStream.connect(...args);
      return reconnectableStream;
    };
  }

  /**
   * @param {Function} createStream - function returning grpc-js/grpc-web stream
   * @param {ReconnectableStreamOptions} options
   */
  constructor(createStream, options = {}) {
    super();

    const opts = { ...defaultOptions, ...options };

    this.logger = opts.logger || { debug: () => {} };

    /**
     * Auto-reconnect interval in millisecond
     * It is needed to automatically reconnect to another DAPI node
     * @type {number}
     */
    this.autoReconnectInterval = opts.autoReconnectInterval;
    this.retryOnErrorDelay = opts.retryOnErrorDelay;

    /**
     * Max amount of retries on error
     */
    this.maxRetriesOnError = opts.maxRetriesOnError;

    /**
     * Current amount of retries on error
     * (Does not have effect if maxRetriesOnError === -1)
     */
    this.retriesOnError = 0;

    /**
     * createStream arguments
     */
    this.args = null;
    /**
     * grpc stream
     */
    this.stream = null;

    /**
     * Function to wrap around to handle interval reconnects and error retries
     */
    this.createStream = createStream;

    /**
     * Reference to setTimeout managed by autoReconnectInterval
     */
    this.reconnectTimeout = null;

    // For mocks
    this.setTimeout = (callback, delay) => setTimeout(callback, delay);
    this.clearTimeout = (reference) => clearTimeout(reference);

    this.connect = this.connect.bind(this);
    this.reconnect = this.reconnect.bind(this);
    this.cancel = this.cancel.bind(this);
    this.errorHandler = this.errorHandler.bind(this);
    this.dataHandler = this.dataHandler.bind(this);
    this.endHandler = this.endHandler.bind(this);
    this.addListeners = this.addListeners.bind(this);
  }

  async connect(...args) {
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug('[ReconnectableStream] Connecting to stream');
    // Memorize current stream args (which can be altered by beforeReconnect logic)
    this.args = args;

    // Create grpc stream
    this.stream = await this.createStream(...this.args);

    // Add event listeners
    this.addListeners();

    if (this.autoReconnectInterval > 0) {
      this.startAutoReconnect();
    }
  }

  startAutoReconnect() {
    if (this.reconnectTimeout) {
      throw new Error('Auto reconnect timeout is already running.');
    }
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug('[ReconnectableStream] Setting reconnect timeout');
    this.reconnectTimeout = this.setTimeout(
      this.reconnect,
      this.autoReconnectInterval,
    );
  }

  reconnect() {
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug('[ReconnectableStream] Try reconnecting to stream');
    if (this.reconnectTimeout) {
      this.reconnectTimeout = null;
      this.stream.cancel();

      let newArgs = this.args || [];
      const updateArgs = (...args) => {
        newArgs = args;
      };

      this.emit(EVENTS.BEFORE_RECONNECT, updateArgs);
      this.connect(...newArgs)
        .catch((connectError) => this.emit(EVENTS.ERROR, connectError));
      // eslint-disable-next-line no-unused-expressions
      this.logger.debug('[ReconnectableStream] Reconnected to stream');
    }
  }

  /**
   * Function that adds EventEmitter style listeners
   * to the stream and also rewires stream cancel function
   * in order to automatically unsubscribe from events
   * @private
   */
  addListeners() {
    this.stream.on(EVENTS.DATA, this.dataHandler);
    this.stream.on(EVENTS.ERROR, this.errorHandler);
    this.stream.on(EVENTS.END, this.endHandler);

    const { cancel } = this.stream;

    // Rewire cancel function in order to
    // unsubscribe from DATA and END events.
    // We don't unsubscribe from ERROR event because it
    // handles `CANCELLED_ON_CLIENT` error after calling .cancel() and must stay
    this.stream.cancel = () => {
      this.stream.removeListener(EVENTS.DATA, this.dataHandler);
      this.stream.removeListener(EVENTS.END, this.endHandler);
      cancel.call(this.stream);
    };
  }

  /**
   * stream.on('data') handler
   * @param data
   */
  dataHandler(data) {
    this.emit(EVENTS.DATA, data);
  }

  /**
   * stream.on('end') handler
   * @private
   */
  endHandler() {
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug('[ReconnectableStream] End handler, stream exists:', !!this.stream);
    if (this.stream) {
      this.stopAutoReconnect();
      this.stream = null;
      this.emit(EVENTS.END);
    }
  }

  /**
   * stream.on('error') handler
   * @private
   * @param e
   */
  errorHandler(e) {
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug(`[ReconnectableStream] Error in stream, code ${e.code}, e:`, e);

    // In case of cancellation nothing has to happen.
    // Do not retry UNKNOWN error code - HACH for grpc-web that ignores following error that happens
    // in a while after stream cancellation
    // Error message:
    // "Response closed without grpc-status (Headers only) {
    //    [Error: Response closed without grpc-status (Headers only)]"
    // TODO: do we need to propagate GrpcErrorCodes.CANCELLED further?
    if (e.code === GrpcErrorCodes.CANCELLED
      || (e.code === GrpcErrorCodes.UNKNOWN && this.stream === null)
    ) {
      // e.code
      this.logger.debug(`[ReconnectableStream] Returning from error handler without restart, error code ${e.code}, e:`);
      return;
    }

    // Retry stream
    this.retryOnError(e);
  }

  /**
   * Manages retry logic
   * @param e
   */
  retryOnError(e) {
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug('[ReconnectableStream] Error handler', e);
    // Stop reconnect timeout if there is one
    this.stopAutoReconnect();

    // Unsubscribe from events in case of the external call
    this.stream.removeListener(EVENTS.END, this.endHandler);
    this.stream.removeListener(EVENTS.DATA, this.dataHandler);

    const canRetry = this.maxRetriesOnError === -1 // Infinite retries
      || this.retriesOnError < this.maxRetriesOnError; // Or less than max limit

    if (canRetry) {
      // Handle 'beforeReconnect` logic
      let newArgs = this.args || [];
      const updateArgsBeforeReconnect = (...args) => {
        newArgs = args;
      };

      // Emit beforeReconnect event in case parent would want to alter arguments
      this.emit(EVENTS.BEFORE_RECONNECT, updateArgsBeforeReconnect);

      // Wait before reconnecting
      wait(this.retryOnErrorDelay)
        .then(() => this.connect(...newArgs))// Reconnect with new args
        .then(() => {
          this.retriesOnError += 1; // Increase amount of current retries
        })
        .catch((connectError) => {
          this.emit(EVENTS.ERROR, connectError); // Or emit error in case retry has failed
        });
    } else {
      // Or simply emit error in case retry attempts were exhausted
      this.emit(EVENTS.ERROR, e);
    }
  }

  /**
   * Stops auto reconnect timeout
   * @private
   */
  stopAutoReconnect() {
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug('[ReconnectableStream] Stopping auto reconnect');
    if (this.reconnectTimeout) {
      this.clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
      // eslint-disable-next-line no-unused-expressions
      this.logger.debug('[ReconnectableStream] Stoped auto reconnect');
    }
  }

  /**
   * Cancels stream.
   * Returns `stream.cancel()` to handle it from parent if needed.
   * (grpc-js cancel() is asynchronous and grpc-web is synchronous)
   * @returns {*}
   */
  cancel() {
    // eslint-disable-next-line no-unused-expressions
    this.logger.debug('[ReconnectableStream] Canceling streams');
    this.stopAutoReconnect();
    // Hack for browsers to properly unsubscribe from ERROR event.
    // (It will continue propagating despite of calling cancel)
    // Ref to unsubscribe from ERROR event
    const { stream } = this;
    setTimeout(() => {
      stream.removeListener(EVENTS.ERROR, this.errorHandler);
      // endHandler
      this.stream = null;
    }, 1000);
    return this.stream.cancel();
  }
}

ReconnectableStream.EVENTS = EVENTS;

module.exports = ReconnectableStream;
