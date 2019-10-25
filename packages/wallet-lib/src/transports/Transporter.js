const DAPIClient = require('@dashevo/dapi-client');
const logger = require('../logger');
const { is, hasProp } = require('../utils/index');

const transportList = {
  dapi: DAPIClient,
};

function isValidTransport(transport) {
  if (is.string(transport)) {
    return hasProp(transportList, transport);
  }
  if (is.obj(transport)) {
    let valid = true;
    const expectedKeys = [
      'getAddressSummary',
      'getTransactionById',
      'getUTXO',
      'sendRawTransaction',
    ];
    expectedKeys.forEach((key) => {
      if (!transport[key]) {
        valid = false;
        logger.error(`Invalid Transporter. Expected key :${key}`);
      }
    });
    return valid;
  }
  return false;
}

class Transporter {
  constructor(transportArg) {
    this.isValid = false;
    this.canConnect = true;
    this.type = null;
    this.transport = null;

    if (is.undef(transportArg)) {
      // eslint-disable-next-line no-param-reassign
      transportArg = 'dapi';
    }
    if (transportArg) {
      let transport = transportArg;
      if (is.string(transportArg)) {
        const loweredTransportName = transportArg.toString().toLowerCase();
        if (Object.keys(transportList).includes(loweredTransportName)) {
          // TODO : Remove me toward release
          if (transportArg === 'dapi') {
            transport = new DAPIClient({
              seeds: [{ service: '18.237.69.61:3000' }],
              timeout: 20000,
              retries: 5,
            });
          } else {
            transport = new transportList[loweredTransportName]();
          }
          this.isValid = isValidTransport(loweredTransportName);
        }
      } else {
        this.isValid = isValidTransport(transportArg);
      }
      this.type = transport.type || transport.constructor.name;
      this.transport = transport;
    }
  }

  handleError(e) {
    const self = this;
    if (!e) {
      return false;
    }
    if (e.code) {
      switch (e.code) {
        case 'ECONNREFUSED':
          if (self.canConnect === true) {
            self.canConnect = false;
            return e;
          }
          break;
        default:
          logger.error('E.code', e.code);
          return e;
      }
    } else if (e && e.response && e.response.data) {
      const { status, error } = e.response.data;
      switch (status) {
        case 429:
          if (error === 'Rate limit exceeded') {
            self.canConnect = false;
            logger.error('Rate limit exceeded');
            return e;
          }
          break;
        default:
          logger.error('e.response.data', e.response.data);
          return e;
      }
    } else {
      throw e;
    }
    return e;
  }

  hasSupportFor(fnName) {
    return typeof this.transport[fnName] === 'function';
  }

  async fetchAndReturn(methodName, params) {
    const self = this;
    if (!this.isValid || !this.canConnect) return false;
    if (!methodName) throw new Error(`Invalid method${methodName}`);
    if (this.hasSupportFor(methodName)) {
      const data = await this.transport[methodName](params).catch(self.handleError.bind(self));
      return data;
    }
    return false;
  }

  async getBestBlockHeight() {
    return this.fetchAndReturn('getBestBlockHeight');
  }

  async getStatus() {
    return this.fetchAndReturn('getStatus');
  }

  async getAddressSummary(address) {
    if (!is.address(address)) throw new Error('Received an invalid address to fetch');
    return this.fetchAndReturn('getAddressSummary', address);
  }

  async getTransaction(txid) {
    if (!is.txid(txid)) throw new Error(`Received an invalid txid to fetch : ${txid}`);
    const data = await this.fetchAndReturn('getTransactionById', txid);
    if (!data) {
      return false;
    }
    if (data.confirmations) {
      delete data.confirmations;
    }
    return data;
  }

  async getUTXO(address) {
    if (!is.address(address)) throw new Error('Received an invalid address to fetch');
    return this.fetchAndReturn('getUTXO', address);
  }

  async subscribeToAddresses(addresses, cb) {
    if (addresses.length > 0 && this.hasSupportFor('subscribeToAddresses')) {
      // todo verify if valid addresses
      // if (!is.address(address)) throw new Error('Received an invalid address to fetch');
      return this.transport.subscribeToAddresses(addresses, cb);
    }
    return false;
  }

  async subscribeToEvent(eventName, cb) {
    if (is.string(eventName) && this.hasSupportFor('subscribeToEvent')) {
      return this.transport.subscribeToEvent(eventName, cb);
    }
    return false;
  }

  disconnect() {
    return (this.transport.closeSocket) ? this.transport.closeSocket() : false;
  }

  updateNetwork(network) {
    if (!this.transport || !this.transport.updateNetwork) {
      throw new Error('Transport does not handle network changes');
    }
    return this.transport.updateNetwork(network);
  }

  getNetwork() {
    if (this.transport) {
      if (this.transport.network) {
        return this.transport.network;
      }
      if (this.transport.getNetwork) {
        return this.transport.getNetwork();
      }
    }
    return null;
  }

  async sendRawTransaction(rawtx, isIs) {
    if (!is.string(rawtx)) throw new Error('Received an invalid rawtx');
    return this.transport.sendRawTransaction(rawtx, isIs);
  }
}

module.exports = Transporter;
