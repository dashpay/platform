const { BIP44_ADDRESS_GAP } = require('../Constants');

const defaultOpts = {
  workerIntervalTime: 1 * 60 * 1000,
};
class BIP44Worker {
  constructor(opts = defaultOpts) {
    this.events = opts.events;
    this.storage = opts.storage;
    this.getAddress = opts.getAddress;
    this.walletId = opts.walletId;
    this.worker = null;
    this.workerPass = 0;
    this.workerRunning = false;
    this.workerIntervalTime = (opts.workerIntervalTime)
      ? opts.workerIntervalTime
      : defaultOpts.workerIntervalTime;
  }

  getNonContinuousIndexes(type = 'external') {
    const nonContinuousIndexes = [];
    const addresses = this.storage.getStore().wallets[this.walletId].addresses[type];
    const paths = Object.keys(addresses);
    if (paths.length > 0) {
      const basePath = paths[0].substring(0, paths[0].length - paths[0].split('/')[5].length);
      const totalNbAddresses = paths.length;
      for (let i = 0, foundAddresses = 0; i < 100 && foundAddresses < totalNbAddresses; i += 1) {
        const path = `${basePath}${i}`;
        if (!addresses[path]) {
          nonContinuousIndexes.push(i);
        }
        foundAddresses += 1;
      }
    }
    return nonContinuousIndexes;
  }

  execWorker() {
    if (this.workerRunning || this.workerPass > 42000) {
      return false;
    }
    this.workerRunning = true;
    const { addresses } = this.storage.store.wallets[this.walletId];
    const externalPaths = Object.keys(addresses.external);
    let externalUnused = 0;

    externalPaths.forEach((path) => {
      const el = addresses.external[path];
      if (el.transactions.length === 0) {
        externalUnused += 1;
      }
    });

    let externalMissingNb = 0;
    if (BIP44_ADDRESS_GAP > externalUnused) {
      externalMissingNb = BIP44_ADDRESS_GAP - externalUnused;
      const { external } = this.storage.store.wallets[this.walletId].addresses;
      const addressKeys = Object.keys(external);
      // console.log(addressKeys)
      const lastElem = external[addressKeys[addressKeys.length - 1]];
      // console.log(BIP44_ADDRESS_GAP, externalUnused, lastElem, addressKeys)

      const addressIndex = (!lastElem) ? -1 : parseInt(lastElem.index, 10);

      for (let i = addressIndex + 1; i < addressIndex + 1 + externalMissingNb; i += 1) {
        this.getAddress(i);
        this.getAddress(i, false);
      }
    }

    // Work as a verifier, will check that index are contiguous or create them
    const nonContinuousIndexes = this.getNonContinuousIndexes();
    nonContinuousIndexes.forEach((index) => {
      this.getAddress(index);
      this.getAddress(index, false);
    });

    this.workerRunning = false;
    this.workerPass += 1;

    this.events.emit('WORKER/BIP44/EXECUTED');
    return true;
  }

  startWorker() {
    const self = this;
    if (this.worker) this.stopWorker();
    // every minutes, check the pool
    this.worker = setInterval(self.execWorker.bind(self), this.workerIntervalTime);
    setTimeout(self.execWorker.bind(self), 100);
    this.events.emit('WORKER/BIP44/STARTED');
  }

  stopWorker() {
    clearInterval(this.worker);
    this.worker = null;
    this.workerPass = 0;
    this.workerRunning = false;
  }
}
module.exports = BIP44Worker;
