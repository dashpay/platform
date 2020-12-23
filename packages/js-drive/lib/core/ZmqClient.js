const { EventEmitter } = require('events');
const zeromq = require('zeromq');

const ZMQ_TOPICS = {
  hashtx: 'hashtx',
  hashtxlock: 'hashtxlock',
  hashblock: 'hashblock',
  rawblock: 'rawblock',
  rawtx: 'rawtx',
  rawtxlock: 'rawtxlock',
  rawtxlocksig: 'rawtxlocksig',
  rawchainlock: 'rawchainlock',
};

const defaultOptions = { topics: ZMQ_TOPICS, maxRetryCount: 20 };

class ZmqClient extends EventEmitter {
  constructor(host, port, options = defaultOptions) {
    super();
    this.subscriberSocket = zeromq.socket('sub');
    this.connectionString = `tcp://${host}:${port}`;
    this.topics = options.topics || [];
    this.maxRetryCount = options.maxRetryCount;
    this.isConnected = false;
    this.resetConnectionFailuresCount();
  }

  resetConnectionFailuresCount() {
    this.connectionFailuresCount = 0;
  }

  /**
   * Starts listening to zmq messages
   * @returns {Promise<any>}
   */
  start() {
    return new Promise((resolve) => {
      this.subscriberSocket.once('connect', () => resolve());
      this.subscriberSocket.once('connect', () => {
        this.emit(ZmqClient.events.CONNECTED);
      });
      this.subscriberSocket.on('connect', () => {
        this.resetConnectionFailuresCount();
      });

      this.initErrorHandlers();
      this.initMessageHandlers();
      this.startMonitor();
      this.subscriberSocket.connect(this.connectionString);
      this.isConnected = true;
    });
  }

  /**
   * @private
   * Starts connection monitor to monitor connection status
   */
  startMonitor() {
    this.subscriberSocket.monitor(500, 0);
  }

  /**
   * @private
   */
  incrementErrorCount() {
    this.connectionFailuresCount += 1;
    if (this.connectionFailuresCount >= this.maxRetryCount) {
      this.emit(ZmqClient.events.MAX_RETRIES_REACHED, `Failed to connect to ZMQ after ${this.maxRetryCount} tries`);
    }
  }

  /**
   * Init connection error handlers. Requires connection monitor to be started
   */
  initErrorHandlers() {
    this.subscriberSocket.on('connect_delay', () => {
      this.emit(ZmqClient.events.CONNECTION_DELAY, 'Dashcore ZMQ connection delay');
      this.incrementErrorCount();
    });
    this.subscriberSocket.on('disconnect', () => {
      this.emit(ZmqClient.events.DISCONNECTED, 'Dashcore ZMQ connection is lost');
      this.incrementErrorCount();
    });
    this.subscriberSocket.on('monitor_error', (error) => {
      this.emit(ZmqClient.events.MONITOR_ERROR, error);
      this.incrementErrorCount();
      setTimeout(() => this.startMonitor(), 1000);
    });
  }

  /**
   * Subscribes to zmq messages
   */
  initMessageHandlers() {
    Object.keys(this.topics).forEach((key) => this.subscriberSocket.subscribe(this.topics[key]));
    this.subscriberSocket.on('message', this.emit.bind(this));
  }

  subscribe(topicName, callback) {
    const isAlreadySubscribed = Object.keys(this.topics).includes(topicName);

    if (!this.isConnected) {
      throw new Error('Socket not connected. Wait until .start() resolves');
    }

    if (!isAlreadySubscribed) {
      this.topics[topicName] = topicName;
      this.subscriberSocket.subscribe(topicName);
    }

    if (callback) {
      this.on(topicName, callback);
    }
  }
}

ZmqClient.events = {
  CONNECTION_DELAY: 'CONNECTION_DELAY',
  DISCONNECTED: 'DISCONNECTED',
  MONITOR_ERROR: 'MONITOR_ERROR',
  ERROR: 'ERROR',
  MAX_RETRIES_REACHED: 'MAX_RETRIES_REACHED',
  CONNECTED: 'CONNECTED',
};

ZmqClient.TOPICS = ZMQ_TOPICS;

module.exports = ZmqClient;
