const { EventEmitter } = require('events');

class ProcessMediator extends EventEmitter {}

ProcessMediator.EVENTS = {
  HISTORICAL_DATA_SENT: 'historicalDataSent',
  TRANSACTION: 'transaction',
  MERKLE_BLOCK: 'merkleBlock',
  CLIENT_DISCONNECTED: 'clientDisconnected',
  HISTORICAL_BLOCK_SENT: 'historicalBlockSent',
  INSTANT_LOCK: 'instantLock',
  MEMPOOL_DATA_SENT: 'memPoolDataSent',
};

module.exports = ProcessMediator;
