const _ = require('lodash');
const { Worker } = require('../');
const { ValidTransportLayerRequired, InvalidTransactionObject } = require('../../errors');
const EVENTS = require('../../EVENTS');
const { UNCONFIRMED_TRANSACTION_STATUS_CODE, SECURE_TRANSACTION_CONFIRMATIONS_NB } = require('../../CONSTANTS');

// eslint-disable-next-line no-underscore-dangle
const _defaultOpts = {
  fetchThreshold: 10 * 60 * 1000,
  workerIntervalTime: 1 * 10 * 1000,
};

class SyncWorker extends Worker {
  constructor(opts = JSON.parse(JSON.stringify(_defaultOpts))) {
    const defaultOpts = JSON.parse(JSON.stringify(_defaultOpts));
    const params = Object.assign({
      name: 'SyncWorker',
      executeOnStart: true,
      firstExecutionRequired: true,
      workerIntervalTime: defaultOpts.workerIntervalTime,
      fetchThreshold: defaultOpts.fetchThreshold,
      dependencies: [
        'storage', 'transport', 'fetchStatus', 'fetchAddressInfo', 'fetchTransactionInfo', 'walletId', 'getBalance', 'getUnusedAddress',
      ],
    }, opts);
    super(params);
    this.isBIP44 = _.has(opts, 'isBIP44') ? opts.isBIP44 : true;

    this.listeners = {
      addresses: [],
    };
    this.bloomfilters = [];
  }

  async execute() {
    // Mostly in order to get our history in sync when we are not listening for new events.
    // We will fetch address summary of our addresses that :
    // - Didn't fetched new update for a bigger period than fetchThreshold (concern all addresses)
    // - If we have an unconfirmed balance
    // - Have more chance to have new tx coming in
    // (typically the result of getUnusedAddress,
    // and preemptively in case of us not receiving the tx from listeners)
    await this.execAddressFetching();

    // We execute a fetching of all the transactions that are unknown
    await this.execTransactionsFetching();


    await this.execAddressListener();
    await this.execAddressBloomfilter();
  }

  async execAddressListener() {
    const self = this;
    const listenerAddresses = [];
    this.listeners.addresses.filter(listener => listenerAddresses.push(listener.address));
    const toPushListener = [];

    const { addresses } = this.storage.getStore().wallets[this.walletId];

    Object.keys(addresses).forEach((walletType) => {
      const walletAddresses = addresses[walletType];
      const walletPaths = Object.keys(walletAddresses);
      if (walletPaths.length > 0) {
        walletPaths.forEach((path) => {
          const address = walletAddresses[path];
          if (!listenerAddresses.includes(address.address)) {
            const listener = {
              address: address.address,
            };
            toPushListener.push(listener);
          }
        });
      }
    });

    toPushListener.forEach((listener) => {
      const listenerObj = Object.assign({}, listener);
      listenerObj.cb = function (event) {
        console.log('Event:', event, listenerObj.address);
      };

      this.listeners.addresses.push(listener);
      // self.transport.subscribeToAddress(listener.address, cb.bind({ listener }));
    });
    const subscribedAddress = this.listeners.addresses.reduce((acc, el) => {
      acc.push(el.address);
      return acc;
    }, []);

    const getTransactionAndStore = async function (tx) {
      if (tx.address && tx.txid) {
        self.storage.addNewTxToAddress(tx, tx.address);
        const transactionInfo = await self.transport.getTransactionById(tx.txid);
        self.storage.importTransactions(transactionInfo);
      }
    };
    await self.transport.subscribeToAddresses(subscribedAddress, getTransactionAndStore);
    return true;
  }

  async execAddressFetching() {
    const self = this;
    const { addresses } = this.storage.getStore().wallets[this.walletId];
    const { fetchAddressInfo } = this;

    const toFetchAddresses = [];

    let prevWasUsed = false;
    Object.keys(addresses).forEach((walletType) => {
      const walletAddresses = addresses[walletType];
      const walletPaths = Object.keys(walletAddresses);

      if (walletPaths.length > 0) {
        walletPaths.forEach((path) => {
          const address = walletAddresses[path];


          const hasUnconfirmedBalance = address.unconfirmedBalanceSat > 0;
          // This will make all last address (the one we got from unconfirmedAddress)
          // to basically check each time
          const isFirstAndUnused = address.used === false && address.index === 0;
          const hasMostChanceToReceiveTx = prevWasUsed === true && address.used === false;
          const hasReachRefreshThreshold = address.fetchedLast < (Date.now() - self.fetchThreshold);
          if (
            isFirstAndUnused
              || hasUnconfirmedBalance
              || hasReachRefreshThreshold
              || hasMostChanceToReceiveTx
          ) {
            toFetchAddresses.push(address);
          }
          prevWasUsed = address.used;
        });
      }
    });
    const promises = [];
    toFetchAddresses.forEach((addressObj) => {
      // We set at false so we don't autofetch utxos. This part will be done in the tx fetching time
      // const p = fetchAddressInfo(addressObj, false)
      const p = fetchAddressInfo(addressObj)
        .catch((e) => {
          if (e instanceof ValidTransportLayerRequired) return false;
          throw e;
        });
      promises.push(p);
    });

    // console.log('fetching addr', toFetchAddresses);
    // console.log(promises)
    const responses = await Promise.all(promises);

    responses.forEach((addrInfo) => {
      try {
        const prev = self.storage.searchAddress(addrInfo.address);
        if (prev.found && (!addrInfo.utxos || Object.keys(addrInfo).length === 0)) {
          // eslint-disable-next-line no-param-reassign
          addrInfo.utxos = prev.result.utxos;
        }
        self.storage.updateAddress(addrInfo, self.walletId);
      } catch (e) {
        self.events.emit(EVENTS.ERROR_UPDATE_ADDRESS, e);
      }
    });
    this.events.emit(EVENTS.FETCHED_ADDRESS, responses);
    return true;
  }

  async execTransactionsFetching() {
    const self = this;
    const { transactions, wallets } = this.storage.getStore();
    const { blockheight, addresses } = wallets[this.walletId];
    const { fetchTransactionInfo } = this;

    const toFetchTransactions = [];
    const unconfirmedThreshold = SECURE_TRANSACTION_CONFIRMATIONS_NB;

    // Parse all addresses and will check if some transaction need to be fetch.
    // This could happen if a tx is yet unconfirmed or if unknown yet.

    Object.keys(addresses).forEach((walletType) => {
      const walletAddresses = addresses[walletType];
      const walletPaths = Object.keys(walletAddresses);
      if (walletPaths.length > 0) {
        walletPaths.forEach((path) => {
          const address = walletAddresses[path];
          const knownsTxId = Object.keys(transactions);
          address.transactions.forEach((txid) => {
            // In case we have a transaction associated to an address but unknown in global level
            if (!knownsTxId.includes(txid)) {
              toFetchTransactions.push(txid);
            } else if (transactions[txid].blockheight === UNCONFIRMED_TRANSACTION_STATUS_CODE) {
              toFetchTransactions.push(txid);
            } else if (blockheight - transactions[txid].blockheight < unconfirmedThreshold) {
              // When the txid is more than -1 but less than 6 conf.
              transactions[txid].spendable = false;
              self.storage.updateTransaction(transactions[txid]);
            } else if (transactions[txid].spendable === false) {
              transactions[txid].spendable = false;
              self.storage.updateTransaction(transactions[txid]);
            }
          });
        });
      }
    });

    const promises = [];

    toFetchTransactions.forEach((transactionObj) => {
      const p = fetchTransactionInfo(transactionObj)
        .then((transactionInfo) => {
          self.storage.importTransaction(transactionInfo);
        }).catch((e) => {
          if (e instanceof ValidTransportLayerRequired) return false;
          if (e instanceof InvalidTransactionObject) return false;
          throw e;
        });
      promises.push(p);
    });

    const resolvedPromised = await Promise.all(promises);
    this.events.emit(EVENTS.FETCHED_TRANSACTIONS, resolvedPromised);
    return true;
  }

  async execAddressBloomfilter() {
    const bloomfilterAddresses = [];
    this.bloomfilters.filter(bloom => bloomfilterAddresses.push(bloom.address));
    const toPushBloom = [];

    toPushBloom.forEach((bloom) => {
      this.bloomfilters.push(bloom);
    });
    return true;
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

module.exports = SyncWorker;
