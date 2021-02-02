const { EventEmitter } = require('events');
const WebSocket = require('ws');

class WsClient extends EventEmitter {
  constructor(options = {}) {
    super();

    const protocol = (options && options.protocol) ? options.protocol.toString() : 'ws';
    const host = (options && options.host) ? options.host.toString() : '0.0.0.0';
    const port = (options && options.port) ? options.port.toString() : '26657';
    const path = (options && options.path) ? options.path.toString() : 'websocket';

    this.url = `${protocol}://${host}:${port}/${path}`;
    this.isConnected = false;
    this.autoReconnectInterval = 1000;
    this.subscribedQueries = new Map();
  }

  /**
   * @private
   * @return <void>
   */
  open() {
    if (this.ws) {
      this.disconnect();
    }

    this.ws = new WebSocket(this.url);

    const reconnect = () => {
      if (this.connectionRetries <= this.maxRetries || this.maxRetries === -1) {
        if (this.maxRetries !== -1) {
          this.connectionRetries += 1;
        }

        setTimeout(this.open.bind(this), this.autoReconnectInterval);
      } else {
        this.disconnect();

        const event = {
          type: 'connect:max_retry_exceeded',
          address: this.url,
        };

        this.emit(event.type, event);
      }
    };

    const onOpenListener = () => {
      this.isConnected = true;

      const event = {
        type: 'connect',
        address: this.url,
      };

      this.emit(event.type, event);

      for (const query of this.subscribedQueries.keys()) {
        this.subscribe(query);
      }
    };

    const onCloseListener = (e) => {
      if (e.code === 1000) { // close normal
        this.disconnect();

        return;
      }

      reconnect();
    };

    const onErrorListener = (e) => {
      switch (e.code) {
        case 'ECONNREFUSED':
          reconnect();
          break;
        default:
          this.disconnect();
          this.emit('error', e);
          break;
      }
    };

    const onMessageListener = (rawData) => {
      const { result } = JSON.parse(rawData);

      if (result !== undefined && Object.keys(result).length > 0) {
        this.emit(result.query, result);
      }
    };

    this.ws.on('open', onOpenListener);
    this.ws.on('close', onCloseListener);
    this.ws.on('error', onErrorListener);
    this.ws.on('message', onMessageListener);
  }

  /**
   *
   * @param {object} connectionOptions
   * @param {number} connectionOptions.maxRetries
   * @return {Promise<void>}
   */
  async connect(connectionOptions = {}) {
    // by default, we don't set any max number of retries
    this.maxRetries = connectionOptions.maxRetries || -1;
    this.connectionRetries = 0;
    this.subscribedQueries.clear();

    return new Promise(async (resolve, reject) => {
      // If a max number of retries is set, we reject when exceeding retry number
      if (this.maxRetries !== -1) {
        this.on('connect:max_retry_exceeded', async () => reject(new Error('Connection dropped. Max retries exceeded.')));
      }

      this.open();

      // We only return socket when we actually established a connection
      this.on('connect', () => resolve());
    });
  }

  /**
   *
   * @return {boolean}
   */
  close() {
    if (this.ws) {
      if (this.isConnected) {
        this.disconnect();
      }

      this.ws = null;
      this.subscribedQueries.clear();

      return true;
    }

    return false;
  }

  disconnect() {
    this.ws.removeAllListeners();
    try {
      this.ws.terminate();
    } catch (e) {
      // do nothing
    }

    this.isConnected = false;
  }

  /**
   *
   * @param {string} query
   */
  subscribe(query) {
    const id = 0;

    const request = {
      jsonrpc: '2.0',
      method: 'subscribe',
      id,
      params: {
        query,
      },
    };

    this.ws.send(JSON.stringify(request));

    const count = this.subscribedQueries.get(query) || 0;
    this.subscribedQueries.set(query, count + 1);
  }

  /**
   *
   * @param {string} query
   */
  unsubscribe(query) {
    const count = this.subscribedQueries.get(query) - 1;

    if (count > 0) {
      this.subscribedQueries.set(query, count);
    } else {
      const id = 0;

      const request = {
        jsonrpc: '2.0',
        method: 'unsubscribe',
        id,
        params: {
          query,
        },
      };

      this.ws.send(JSON.stringify(request));

      this.subscribedQueries.delete(query);
    }
  }
}

module.exports = WsClient;
