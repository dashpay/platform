const EventEmitter = require('events');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const defaultOptions = {
  reconnectTimeoutDelay: 50000,
};

const EVENTS = {
  DATA: 'data',
  ERROR: 'error',
  END: 'end',
  BEFORE_RECONNECT: 'beforeReconnect',
};

/**
 * Stream that provides auto-reconnect functionality
 */
class ReconnectableStream extends EventEmitter {
  static create(fn, options) {
    return async (...args) => {
      const reconnectableStream = new ReconnectableStream(fn, options);
      await reconnectableStream.connect(...args);
      return reconnectableStream;
    };
  }

  constructor(streamFunction, options = {}) {
    super();
    this.stream = null;
    this.streamFunction = streamFunction;
    this.reconnectTimeout = null;

    this.options = { ...defaultOptions, ...options };

    /**
     * Stream function arguments
     */
    this.args = null;

    this.connect = this.connect.bind(this);
    this.reconnect = this.reconnect.bind(this);
    this.cancel = this.cancel.bind(this);
    this.destroy = this.destroy.bind(this);
    this.errorHandler = this.errorHandler.bind(this);
    this.dataHandler = this.dataHandler.bind(this);
    this.endHandler = this.endHandler.bind(this);
    this.addListeners = this.addListeners.bind(this);
  }

  async connect(...args) {
    this.args = args;
    this.stream = await this.streamFunction(...this.args);
    this.addListeners();

    this.reconnectTimeout = setTimeout(
      this.reconnect,
      this.options.reconnectTimeoutDelay,
    );
  }

  reconnect() {
    if (this.reconnectTimeout) {
      this.reconnectTimeout = null;
      this.stream.cancel();

      let newArgs = this.args;
      const updateArgs = (...args) => {
        newArgs = args;
      };

      this.emit(EVENTS.BEFORE_RECONNECT, updateArgs);
      this.connect(...newArgs)
        .catch((connectError) => this.emit(EVENTS.ERROR, connectError));
    }
  }

  /**
   * @private
   */
  addListeners() {
    this.stream.on(EVENTS.DATA, this.dataHandler);
    this.stream.on(EVENTS.ERROR, this.errorHandler);
    this.stream.on(EVENTS.END, this.endHandler);

    const { cancel } = this.stream;

    this.stream.cancel = () => {
      this.stream.removeListener(EVENTS.DATA, this.dataHandler);
      this.stream.removeListener(EVENTS.END, this.endHandler);
      cancel.call(this.stream);
    };
  }

  dataHandler(data) {
    this.emit(EVENTS.DATA, data);
  }

  /**
   * @private
   */
  endHandler() {
    this.stopReconnectTimeout();
    this.stream = null;
    this.emit(EVENTS.END);
  }

  /**
   * @private
   * @param e
   */
  errorHandler(e) {
    this.stream.removeListener(EVENTS.ERROR, this.errorHandler);
    if (e.code === GrpcErrorCodes.CANCELLED) {
      return;
    }
    this.stopReconnectTimeout();
    this.emit(EVENTS.ERROR, e);
  }

  /**
   * @private
   */
  stopReconnectTimeout() {
    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }
  }

  cancel() {
    this.stopReconnectTimeout();
    return this.stream.cancel();
  }

  destroy(e) {
    this.stream.destroy(e);
  }
}

ReconnectableStream.EVENTS = EVENTS;

module.exports = ReconnectableStream;
