/* eslint-disable no-restricted-syntax */
const _ = require('lodash');
const { Worker } = require('../../index');

// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  workerIntervalTime: 10 * 1000,
};

/**
 * SyncWorker is responsible for keeping the Wallet in Sync with the network informations.
 * Information kept in sync such as : addresses utxos,
 * Chain information are handled by ChainPlugin
 *
 */
class SyncWorker extends Worker {
  /**
   * Create the syncWorker instance
   * @param opts
   */
  constructor(opts = JSON.parse(JSON.stringify(_defaultOpts))) {
    const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));
    const params = {
      name: 'SyncWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      workerIntervalTime: defaultOpts.workerIntervalTime,
      fetchThreshold: defaultOpts.fetchThreshold,
      dependencies: [
        'storage', 'transporter', 'fetchStatus', 'getTransaction', 'fetchAddressInfo', 'walletId', 'getUnusedAddress',
      ],
      ...opts,
    };
    super(params);
    this.isBIP44 = _.has(opts, 'isBIP44') ? opts.isBIP44 : true;

    this.listeners = {
      addresses: [],
    };
    this.bloomfilters = [];
  }

  async onStart() {
    // When SyncWorker.onStart gets executed. BIP44 Worker (if applicable), will already have ran.
    // At this stage, we just generated our local address pool
    // We therefore need to do a first sync-up (for balance reason).
    // Because we listen to the event. We need to know if we had fetched tx before releasing onStart
    await this.initialSyncUp();
  }

  async execute() {
    // We will needed to update the transporter about the addresses we need to listen
    // which is something that can change over the course of the use of the lib.
    const addrList = this.getAddressListToSync().map((addr) => addr.address);
    await this.transporter.subscribeToAddressesTransactions(addrList);
  }
}

SyncWorker.prototype.announce = require('./announce');
SyncWorker.prototype.getAddressListToSync = require('./getAddressListToSync');
SyncWorker.prototype.initialSyncUp = require('./initialSyncUp');

module.exports = SyncWorker;
