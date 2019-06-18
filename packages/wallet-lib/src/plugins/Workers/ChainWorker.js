const { Worker } = require('../');
const { ValidTransportLayerRequired } = require('../../errors');
const EVENTS = require('../../EVENTS');

// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  workerIntervalTime: 0,
  workerMaxPass: 1, // Unique execution on start
  firstExecutionRequired: true,
  executeOnStart: true,
};

class ChainWorker extends Worker {
  constructor(opts = JSON.parse(JSON.stringify(_defaultOpts))) {
    const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));
    const params = Object.assign({
      name: 'ChainWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      workerIntervalTime: defaultOpts.workerIntervalTime,
      workerMaxPass: defaultOpts.workerMaxPass,
      dependencies: [
        'storage',
        'transport',
        'fetchStatus',
        'fetchAddressInfo',
        'fetchTransactionInfo',
        'walletId',
        'getBalance',
      ],
    }, opts);
    super(params);
  }

  async execBlockListener() {
    const self = this;
    const cb = async function (block) {
      const { network } = self.storage.store.wallets[self.walletId];
      self.storage.store.chains[network.toString()].blockheight += 1;
      self.announce(EVENTS.BLOCK, block);
      self.announce(
        EVENTS.BLOCKHEIGHT_CHANGED,
        self.storage.store.chains[network.toString()].blockheight,
      );
    };
    if (self.transport.isValid) {
      self.transport.subscribeToEvent(EVENTS.BLOCK, cb);
    }
  }

  async execStatusFetch() {
    try {
      const res = await this.fetchStatus();
      if (!res) {
        return false;
      }
      const { blocks } = res;
      const { network } = this.storage.store.wallets[this.walletId];
      this.storage.store.chains[network.toString()].blockheight = blocks;
      this.announce(EVENTS.BLOCKHEIGHT_CHANGED, blocks);
      return true;
    } catch (e) {
      if (e instanceof ValidTransportLayerRequired) {
        console.log('invalid');
      }
      return e;
    }
  }

  async execute() {
    await this.execStatusFetch();
    await this.execBlockListener();
  }

  announce(type, el) {
    switch (type) {
      case EVENTS.BLOCK:
        this.events.emit(EVENTS.BLOCK, el);
        break;
      case EVENTS.BLOCKHEIGHT_CHANGED:
        this.events.emit(EVENTS.BLOCKHEIGHT_CHANGED, el);
        break;
      default:
        this.events.emit(type, el);
        console.warn('Not implemented, announce of ', type, el);
    }
  }
}

module.exports = ChainWorker;
