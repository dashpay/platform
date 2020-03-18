const { EventEmitter2: EventEmitter } = require('eventemitter2');

class BaseTransporter extends EventEmitter {
  constructor(props) {
    super(props);
    this.type = props.type;
    this.state = {
      block: null,
      blockHeaders: null,
      // Executors are Interval
      executors: {
        blocks: null,
        blockHeaders: null,
        addresses: null,
      },
      addressesTransactionsMap: {},
      subscriptions: {
        addresses: {},
      },
    };
  }
}
BaseTransporter.prototype.announce = require('./methods/announce');
BaseTransporter.prototype.disconnect = require('./methods/disconnect');
BaseTransporter.prototype.getAddressSummary = require('./methods/getAddressSummary');
BaseTransporter.prototype.getBestBlockHash = require('./methods/getBestBlockHash');
BaseTransporter.prototype.getBestBlockHeight = require('./methods/getBestBlockHeight');
BaseTransporter.prototype.getBlockByHash = require('./methods/getBlockByHash');
BaseTransporter.prototype.getBlockByHeight = require('./methods/getBlockByHeight');
BaseTransporter.prototype.getBlockHeaderByHash = require('./methods/getBlockHeaderByHash');
BaseTransporter.prototype.getBlockHeaderByHeight = require('./methods/getBlockHeaderByHeight');
BaseTransporter.prototype.getStatus = require('./methods/getStatus');
BaseTransporter.prototype.getTransaction = require('./methods/getTransaction');
BaseTransporter.prototype.getUTXO = require('./methods/getUTXO');
BaseTransporter.prototype.sendTransaction = require('./methods/sendTransaction');
BaseTransporter.prototype.subscribeToAddressesTransactions = require('./methods/subscribeToAddressesTransactions');
BaseTransporter.prototype.subscribeToBlockHeaders = require('./methods/subscribeToBlockHeaders');
BaseTransporter.prototype.subscribeToBlocks = require('./methods/subscribeToBlocks');

module.exports = BaseTransporter;
