const { EventEmitter } = require('events');

class ProcessMediator extends EventEmitter {}

ProcessMediator.EVENTS = {
  HISTORICAL_DATA_SENT: 'historicalDataSent',
  BLOCK_HEADERS: 'blockHeaders',
  CHAIN_LOCK_SIGNATURE: 'chainLockSignature',
  HISTORICAL_BLOCK_HEADERS_SENT: 'historicalBlockHeadersSent',
  CLIENT_DISCONNECTED: 'clientDisconnected',
};

module.exports = ProcessMediator;
