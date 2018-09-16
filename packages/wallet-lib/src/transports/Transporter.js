const { is, hasProp } = require('../utils/index');
const InsightClient = require('../transports/Insight/insightClient');
const DAPIClient = require('../transports/DAPI/DapiClient');


const transportList = {
  Insight: InsightClient,
  DAPIClient,
};

function isValidTransport(transport) {
  if (is.string(transport)) {
    return hasProp(transportList, transport);
  } if (is.obj(transport)) {
    let valid = true;
    const expectedKeys = [
      'getAddressSummary',
      'getTransaction',
      'getUTXO',
      'subscribeToAddresses',
      'closeSocket',
      'sendRawTransaction',
    ];
    expectedKeys.forEach((key) => {
      if (!transport[key]) {
        valid = false;
      }
    });
    return valid;
  }
  return false;
}

class Transporter {
  constructor(transportArg) {
    this.valid = false;
    this.type = null;
    this.transport = null;

    if (transportArg) {
      let transport = transportArg;
      if (is.string(transportArg) && Object.keys(transportList).includes(transportArg)) {
        transport = transportList[transportArg];
      }
      this.valid = isValidTransport(transportArg);
      this.type = transport.type || transport.constructor.name;
      this.transport = transport;
    }
  }

  async getStatus() {
    const data = await this.transport
      .getStatus()
      .catch((err) => {
        throw new Error(err);
      });
    return data.info;
  }

  async getAddressSummary(address) {
    if (!is.address(address)) throw new Error('Received an invalid address to fetch');
    const data = await this
      .transport
      .getAddressSummary(address)
      .catch((err) => {
        throw new Error(err);
      });
    return data;
  }

  async getTransaction(txid) {
    if (!is.txid(txid)) throw new Error(`Received an invalid txid to fetch : ${txid}`);
    const data = await this.transport.getTransaction(txid).catch((err) => {
      throw new Error(err);
    });
    if (data.confirmations) {
      delete data.confirmations;
    }
    return data;
  }

  async getUTXO(address) {
    if (!is.address(address)) throw new Error('Received an invalid address to fetch');
    const data = await this.transport.getUTXO(address).catch((err) => {
      throw new Error(err);
    });
    return data;
  }

  async subscribeToAddresses(addresses, cb) {
    if (addresses.length > 0) {
      // todo verify if valid addresses
      // if (!is.address(address)) throw new Error('Received an invalid address to fetch');
      return this.transport.subscribeToAddresses(addresses, cb);
    }
    return false;
  }

  async subscribeToEvent(eventName, cb) {
    if (is.string(eventName)) {
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
    return this.transport.network;
  }

  async sendRawTransaction(rawtx, isIs) {
    if (!is.string(rawtx)) throw new Error('Received an invalid rawtx');
    return this.transport.sendRawTransaction(rawtx, isIs);
  }
}
module.exports = Transporter;
