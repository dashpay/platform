const EventEmitter = require('events');
const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const defaultOptions = {
  reconnectTimeoutDelay: 5000,
};

const EVENTS = {
  DATA: 'data',
  ERROR: 'error',
  END: 'end',
  BEFORE_RECONNECT: 'beforeReconnect',
};

// TODO: test
/**
 * Stream that provides auto-reconnect functionality
 */
class DAPIStream extends EventEmitter {
  static create(fn) {
    return async (...args) => {
      const dapiStream = new DAPIStream(fn, args);
      await dapiStream.connect(...args);
      return dapiStream;
    };
  }

  constructor(streamFunction, options = {}) {
    super();
    this.stream = null;
    this.streamFunction = streamFunction;
    this.reconnectTimeout = null;
    /**
     * A supplemental flag to handle deliberate stream cancellations
     *
     * @type {boolean}
     */
    this.reconnectingAfterTimeout = false;
    this.options = { ...defaultOptions, ...options };

    /**
     * Stream function arguments
     */
    this.args = null;

    this.connect = this.connect.bind(this);
    this.cancel = this.cancel.bind(this);
    this.destroy = this.destroy.bind(this);
    this.errorHandler = this.errorHandler.bind(this);
    this.endHandler = this.endHandler.bind(this);
    this.addListeners = this.addListeners.bind(this);
    this.removeListeners = this.removeListeners.bind(this);
  }

  async connect(...args) {
    this.removeListeners();

    this.args = args;
    this.stream = await this.streamFunction(...this.args);
    this.addListeners();
    this.reconnectTimeout = setTimeout(() => {
      this.reconnectingAfterTimeout = true;
      this.reconnectTimeout = null;
      this.stream.cancel();
    }, this.options.reconnectTimeoutDelay);
  }

  /**
   * @private
   */
  addListeners() {
    this.stream.on(EVENTS.DATA, (data) => this.emit(EVENTS.DATA, data));
    this.stream.on(EVENTS.ERROR, this.errorHandler);
    this.stream.on(EVENTS.END, this.endHandler);
  }

  /**
   * @private
   */
  endHandler() {
    this.removeListeners();
    this.stopReconnectTimeout();
    this.stream = null;
    this.emit(EVENTS.END);
  }

  /**
   * @private
   * @param e
   */
  errorHandler(e) {
    if (e.code === GrpcErrorCodes.CANCELLED) {
      this.removeListeners();
      if (this.reconnectingAfterTimeout) {
        this.reconnectingAfterTimeout = false;

        let newArgs = this.args;
        const updateArgs = (...args) => {
          newArgs = args;
        };

        this.emit(EVENTS.BEFORE_RECONNECT, updateArgs);
        this.connect(...newArgs)
          .catch((connectError) => this.emit(EVENTS.ERROR, connectError));
      } else {
        this.endHandler();
      }
    } else {
      this.emit(EVENTS.ERROR, e);
    }
  }

  /**
   * @private
   */
  removeListeners() {
    if (this.stream) {
      this.stream.removeAllListeners(EVENTS.DATA);
      this.stream.removeAllListeners(EVENTS.ERROR);
      this.stream.removeAllListeners(EVENTS.END);
    }
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
    this.stream.cancel();
    this.stopReconnectTimeout();
  }

  destroy(e) {
    this.stream.destroy(e);
    this.stopReconnectTimeout();
  }
}

DAPIStream.EVENTS = EVENTS;

module.exports = DAPIStream;
