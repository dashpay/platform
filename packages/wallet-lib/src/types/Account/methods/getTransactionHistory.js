const _ = require('lodash');
const { dashToDuffs, duffsToDash } = require('../../../utils');

// Will filter out transaction that are not concerning us
// (which can happen in the case of multiple account in store)
const getFilteredTransactions = async function (/* storage, walletId, accountIndex */) {
  /**
   * From transaction's hash, we would need to be able to find the time of such execution.
   * Previously we used 'confirmations' value to estimate the height block where it would
   * be included.
   * This has been removed, and there is no way for us to easily get the block height
   * or hash from a tx.
   * In order to support this feature, it would require us to have the whole raw block set
   * in order to find a tx in a block.
   */
  return new Error('Removed feature - unable to calculate time between transactions');
  /*
  const txids = [];
  const txs = [];
  const store = storage.getStore();
  const { transactions } = store;
  const { addresses } = store.wallets[walletId];

  const isHDWallet = !((Object.keys(addresses.misc).length > Object.keys(addresses.external)));

  if (isHDWallet) {
    const { external, internal } = addresses;

    _.each({ ...external, ...internal }, (address) => {
      if (!isHDWallet
          || (isHDWallet && parseInt(address.path.split('/')[3], 10) === accountIndex)
      ) {
        address.transactions.forEach((txid) => {
          if (!txids.includes(txid)) {
            txids.push(txid);
          }
        });
      }
    });
  } else {
    const { misc } = addresses;
    _.each(misc, (address) => {
      address.transactions.forEach((txid) => {
        if (!txids.includes(txid)) {
          txids.push(txid);
        }
      });
    });
  }

  // for (const transactionId of txids) {
  //   const tx = transactions[transactionId];
  //   const {
  //     hash: txid, nLockTime, vin, vout,
  //   } = tx;
  //   const time = storage.getBlockHeader(nLockTime);
  //
  //   console.log({ time });
  // }
  _.each(txids, (transactionId) => {
    const tx = transactions[transactionId];

    const {
      hash: txid, nLockTime, vin, vout,
    } = tx;

    const block = storage.getBlockHeader(txid);
    // const time = storage.getBlockHeader(txid);

    txs.push({
      txid,
      // fees:tx._estimateFee(),
      vin,
      vout,
      time,
    });
  });

  return txs;
   */
};

const sortByTimeDesc = (a, b) => (b.time - a.time);

const considerValue = (list, address, valueSat) => {
  let found = false;
  _.each(list, (el) => {
    if (el.address === address) {
      // eslint-disable-next-line no-param-reassign
      el.valueSat += valueSat;
      found = true;
    }
  });

  if (!found) {
    list.push({ address, valueSat });
  }
};

/**
 * Get all the transaction history already formated
 * @return {Promise<any[]>}
 */
async function getTransactionHistory() {
  const transactionHistory = [];

  const { walletId } = this;
  const accountIndex = this.index;
  const store = this.storage.getStore();

  const txs = await getFilteredTransactions(this.storage, walletId, accountIndex);

  const { addresses } = store.wallets[walletId];
  const isHDWallet = !((Object.keys(addresses.misc).length > Object.keys(addresses.external)));

  const predicate = (addr) => ({ address: addr.address });
  // eslint-disable-next-line consistent-return
  const filterInAccountPredicate = (addr) => {
    if ((isHDWallet && parseInt(addr.path.split('/')[3], 10) === accountIndex)) {
      return addr;
    }
  };
  // eslint-disable-next-line consistent-return
  const filterOutAccountPredicate = (addr) => {
    if ((isHDWallet && parseInt(addr.path.split('/')[3], 10) !== accountIndex)) {
      return addr;
    }
  };
  const changeAddresses = (isHDWallet)
    ? _.map(_.filter(addresses.internal, filterInAccountPredicate), predicate)
    : _.map(addresses.misc, predicate);
  const changeAddressList = changeAddresses.map((addr) => addr.address, []);

  const externalAddresses = (isHDWallet)
    ? _.map(_.filter(addresses.external, filterInAccountPredicate), predicate)
    : _.filter(addresses.misc, predicate);
  const externalAddressesList = externalAddresses.map((addr) => addr.address, []);

  const mergedAddresses = { ...addresses.external, ...addresses.internal };

  const otherAccountAddresses = (isHDWallet)
    ? _.map(_.filter(mergedAddresses, filterOutAccountPredicate), predicate)
    : [];
  const otherAccountAddressesList = otherAccountAddresses
    .map((addr) => addr.address, []);

  const determinateTransactionMetaData = (tx) => {
    const from = [];
    const to = [];

    const meta = {
      vout: {
        external: {},
        change: {},
        unknown: {},
        otherAccount: {},
      },
      vin: {
        external: {},
        change: {},
        unknown: {},
        otherAccount: {},
      },
    };

    const determineType = (isChange, isExternal, isOtherAccount) => {
      if (isExternal) return 'external';
      if (isChange) return 'change';
      return (isOtherAccount) ? 'otherAccount' : 'unknown';
    };

    _.each(tx.vout, (vout) => {
      if (vout.scriptPubKey && vout.scriptPubKey.addresses) {
        const address = vout.scriptPubKey.addresses[0];
        const isChange = changeAddressList.includes(address);
        const isExternal = externalAddressesList.includes(address);
        const isOtherAccount = otherAccountAddressesList.includes(address);

        const el = { address, valueSat: dashToDuffs(parseFloat(vout.value)) };
        const type = determineType(isChange, isExternal, isOtherAccount);

        if (!meta.vout[type][address]) meta.vout[type][address] = el;
        else meta.vout[type][address].valueSat += el.valueSat;
      }
    });


    _.each(tx.vin, (vin) => {
      if (vin.addr) {
        const address = vin.addr;
        const isChange = changeAddressList.includes(address);
        const isExternal = externalAddressesList.includes(address);
        const isOtherAccount = otherAccountAddressesList.includes(address);

        const el = { address, valueSat: vin.valueSat };
        const type = determineType(isChange, isExternal, isOtherAccount);

        if (!meta.vin[type][address]) meta.vin[type][address] = el;
        else meta.vin[type][address].valueSat += el.valueSat;
      }
    });

    const nbOfExternalVin = Object.keys(meta.vin.external).length;
    const nbOfUnknwonVin = Object.keys(meta.vin.unknown).length;
    const nbOfChangeVin = Object.keys(meta.vin.change).length;
    const nbOfOtherAccountVin = Object.keys(meta.vin.otherAccount).length;

    const nbOfExternalVout = Object.keys(meta.vout.external).length;
    const nbOfUnknwonVout = Object.keys(meta.vout.unknown).length;
    // const nbOfChangeVout = Object.keys(meta.vout.change).length;
    const nbOfOtherAccountVout = Object.keys(meta.vout.otherAccount).length;

    let type;
    if (nbOfOtherAccountVin > 0 && nbOfChangeVin + nbOfExternalVin + nbOfUnknwonVin === 0) {
      // When this account is recipient of an inbetween account movement
      type = 'moved_account';

      _.each(meta.vout.external, (el) => {
        considerValue(to, el.address, (parseFloat(el.valueSat)));
      });

      _.each(meta.vin.otherAccount, (el) => {
        considerValue(from, el.address, (parseFloat(el.valueSat)));
      });
    } else if (nbOfOtherAccountVout > 0 || nbOfOtherAccountVin > 0) {
      // When the account is sender of an inbetween account movement
      type = 'moved_account';


      if (nbOfOtherAccountVout > 0) {
        _.each(meta.vout.otherAccount, (el) => {
          considerValue(to, el.address, (parseFloat(el.valueSat)));
        });
      }
      if (nbOfOtherAccountVin > 0) {
        _.each(meta.vout.otherAccount, (el) => {
          considerValue(to, el.address, (parseFloat(el.valueSat)));
        });
      }

      _.each([meta.vin.external, meta.vin.change, meta.vin.otherAccount], (metaType) => {
        _.each(metaType, (el) => {
          considerValue(from, el.address, (parseFloat(el.valueSat)));
        });
      });
    } else if (nbOfExternalVin + nbOfChangeVin === 0) {
      // When we don't know any of the vin address then we received some
      type = 'receive';

      _.each(meta.vout.external, (el) => {
        considerValue(to, el.address, (parseFloat(el.valueSat)));
      });
      _.each(meta.vin.unknown, (el) => {
        considerValue(from, el.address, (parseFloat(el.valueSat)));
      });
      _.each(meta.vin.otherAccount, (el) => {
        considerValue(from, el.address, (parseFloat(el.valueSat)));
      });
    } else if (nbOfUnknwonVout === 0) {
      // If it's nothing above, and we don't have any unknown output, then it's internal move
      type = 'moved';

      if (nbOfExternalVout > 0) {
        _.each(meta.vout.external, (el) => {
          considerValue(to, el.address, (parseFloat(el.valueSat)));
        });
      }
      _.each([meta.vin.external, meta.vin.change], (metaType) => {
        _.each(metaType, (el) => {
          considerValue(from, el.address, (parseFloat(el.valueSat)));
        });
      });
    } else {
      type = 'sent';
      _.each(meta.vout.unknown, (el) => {
        considerValue(to, el.address, (parseFloat(el.valueSat)));
      });
      _.each([meta.vin.external, meta.vin.change], (metaType) => {
        _.each(metaType, (el) => {
          considerValue(from, el.address, (parseFloat(el.valueSat)));
        });
      });
      // As for a single wallet this will be a duplicate
      if (type === 'hdwallet') {
        _.each(meta.vin.change, (el) => {
          considerValue(from, el.address, (parseFloat(el.valueSat)));
        });
      }
    }

    return { type, to, from };
  };

  _.each(txs, (tx) => {
    const { txid, time } = tx;
    const { type, to, from } = determinateTransactionMetaData(tx);

    const cleanUpPredicate = (val) => ({
      address: val.address,
      amount: duffsToDash(val.valueSat).toString(),
      valueSat: val.valueSat,
    });
    const transaction = {
      txid,
      time,
      to: to.map(cleanUpPredicate),
      from: from.map(cleanUpPredicate),
      // fees,
      type,
    };

    transactionHistory.push(transaction);
  });

  return transactionHistory.sort(sortByTimeDesc);
}

module.exports = getTransactionHistory;
