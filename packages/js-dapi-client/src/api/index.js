const { request } = require('./DAPIClient');

const api = {
  address: {
    getUTXO: address => request('getUTXO', [address]),
    getBalance: address => request('getBalance', [address]),
  },
  user: {
    getUser: usernameOrRegTxId => request('getUser', [usernameOrRegTxId]),
  },
  transaction: {
    sendRaw: rawTx => request('sendRawTransaction', [rawTx]),
  },
  stateTransition: {
    sendRaw(rawTransition, dataPacket) {
      return request('sendRawTransition', [rawTransition, dataPacket]);
    },
  },
  block: {
    getBestBlockHeight: () => request('getBestBlockHeight', []),
    getBlockHash: blockHeight => request('getBlockHash', [blockHeight]),
  },
};

module.exports = api;
