const Dashcore = require('@dashevo/dashcore-lib');
const axios = require('axios');
const io = require('socket.io-client');
const { is, dashToDuffs } = require('../../utils/index');
// Here to avoid asking to much to the network when doing a nodemon for the tests.
// Probably will require to mock the test part.

const defaultOpts = {
  uris: {
    insight: {
      livenet: 'https://insight.dashevo.org/insight-api-dash',
      testnet: 'https://testnet-insight.dashevo.org/insight-api-dash',
    },
    sockets: {
      livenet: 'https://insight.dashevo.org/',
      testnet: 'https://testnet-insight.dashevo.org/',
    },
  },
  network: 'testnet',
  useSocket: true,
};
/**
 * Temporary class to perform request on insight instead of DAPI
 */
class InsightClient {
  constructor(opts = defaultOpts) {
    this.listeners = {};
    this.type = this.constructor.name;
    this.useSocket = (opts.useSocket) ? opts.useSocket : defaultOpts.useSocket;

    this.setNetwork((opts.network) ? opts.network : defaultOpts.network);
    this.setInsightURI((opts.uri) ? opts.uri : defaultOpts.uris.insight[this.network]);
    this.setSocketURI((opts.socketUri) ? opts.socketUri : defaultOpts.uris.sockets[this.network]);
  }

  setNetwork(network) {
    this.network = (network) ? Dashcore.Networks[network] : Dashcore.Networks.testnet;
    return true;
  }

  setSocketURI(uri) {
    this.socketUri = (uri) || (defaultOpts.uris.sockets[this.network]);
    if (this.useSocket) {
      if (this.socket) {
        this.closeSocket();
      }
      this.socket = io(this.socketUri, { transports: ['websocket'] });
      this.socket.emit('subscribe', 'inv');
      this.socket.on('connect', () => console.log('Socket connected!'));
      this.socket.on('event', event => console.log('event', event));
      this.socket.on('disconnect', disconnect => console.log('disconnect', disconnect));
      this.socket.on('error', error => console.log('Socket err', error));

      // this.subscribeToEvent('block', tx => console.log('txlock', tx));
      // this.subscribeToEvent('txlock', tx => console.log('txlock', tx));
      // this.subscribeToEvent('tx', tx => console.log('tx', tx));
    }
    return true;
  }

  setInsightURI(uri) {
    this.insightUri = (uri) || defaultOpts.uris.insight[this.network];
    return true;
  }

  updateNetwork(network) {
    return this.setNetwork(network) && this.setSocketURI() && this.setInsightURI();
  }

  closeSocket() {
    if (this.useSocket) {
      this.socket.close();
    }
  }

  async getAddressSummary(address) {
    const url = `${this.insightUri}/addr/${address}`;
    return axios
      .get(url)
      .then(res => res.data)
      .catch((err) => {
        throw err;
      });
  }


  async getTransaction(transactionid) {
    const res = await axios.get(`${this.insightUri}/tx/${transactionid}`);
    if (res.data) {
      if (res.data.fees && is.float(res.data.fees)) {
        res.data.fees = dashToDuffs(res.data.fees);
      }
      if (res.data.confirmations) {
        delete res.data.confirmations;
      }
    }


    return res.data;
  }

  async getStatus() {
    const res = await axios.get(`${this.insightUri}/status`);
    return res.data;
  }

  async getUTXO(address) {
    const res = await axios.get(`${this.insightUri}/addr/${address}/utxo`);
    return res.data;
  }

  async sendRawTransaction(rawtx, isIs = false) {
    const url = (isIs) ? `${this.insightUri}/tx/sendix` : `${this.insightUri}/tx/send`;
    return axios
      .post(url, { rawtx })
      .then(res => res.data)
      .catch((err) => {
        console.log(err);
        throw new Error(err);
      });
  }

  subscribeToEvent(eventName, cb) {
    if (this.useSocket) {
      if (this.listeners[eventName]) {
        return false;
      }
      this.socket.emit('subscribe', eventName);
      const listener = this.socket.on(eventName, cb);
      this.listeners[eventName] = {
        type: eventName,
        listener,
        setTime: Date.now(),
      };
      console.log('Subscribed to event :', eventName);
      return true;
    }
    return false;
  }

  unsubscribeFromEvent(eventName) {
    if (this.listeners[eventName]) {
      this.clearListener(this.listeners[eventName].listener);
      delete this.listeners[eventName];
    }
  }

  clearListener(listener) {
    this.socket.emit('unsubscribe', listener.type);
    this.socket.removeListener(listener.listener);
  }


  subscribeToAddresses(addresses, cb) {
    if (this.useSocket) {
      const eventName = 'dashd/addresstxid';
      if (this.listeners[eventName]) {
        const oldListener = this.listeners[eventName];
        if (JSON.stringify(addresses) === JSON.stringify(oldListener.addresses)) {
          return false; // Same addresses, everything is already set
        }
        this.clearListener(oldListener);
      }
      this.socket.emit('subscribe', eventName, addresses);
      const listener = this.socket.on(eventName, cb);
      this.listeners[eventName] = {
        type: eventName,
        listener,
        addresses,
        emitter: true,
        setTime: Date.now(),
      };
      console.log('Subscribed to ', eventName);
      return true;
    }
    return false;
  }
}
module.exports = InsightClient;
