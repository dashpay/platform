const { merge } = require('lodash');
const { InstantLock, Transaction, BlockHeader } = require('@dashevo/dashcore-lib');
const { hasMethod } = require('../../../utils');

const mergeHelper = (initial = {}, additional = {}) => merge(initial, additional);
const { REHYDRATE_STATE_FAILED, REHYDRATE_STATE_SUCCESS } = require('../../../EVENTS');
const logger = require("../../../logger");

const storeTypesSchema = {
  "transactions": {
    "*": Transaction
  },
  "instantLocks": {
    "*": InstantLock
  },
  "wallets": {
    "*": {
      "addresses": {
        "external": {
          "*": {
            "utxos": {
              "*": function (item) {
                // TODO: resolve type inconsistency for address utxos
                try {
                  return new Transaction.UnspentOutput(item);
                } catch (e) {
                  return new Transaction.Output(item);
                }
              }
            }
          }
        },
        "internal": {
          "*": {
            "utxos": {
              "*": function (item) {
                // TODO: resolve type inconsistency for address utxos
                try {
                  return new Transaction.UnspentOutput(item);
                } catch (e) {
                  return new Transaction.Output(item);
                }
              }
            }
          }
        }
      }
    }
  },
  "chains": {
    "*": {
      "blockHeaders": {
        "*": BlockHeader
      }
    }
  }
}

const rehydrateItem = (item, name) => {
  if (!item) {
    return null;
  }

  const itemSchema = storeTypesSchema[name];

  if (!itemSchema) {
    logger.silly(`Storage.rehydrateState(): no types schema defined for item "${name}"`);
    return item;
  }

  const ensureTypes = (schema, item) => {
    Object.entries(schema).forEach(([schemaKey, schemaValue]) => {
      if (schemaValue.constructor.name !== "Object") {
        const clazz = schemaValue;
        if (schemaKey === "*") {
          Object.keys(item).forEach((itemKey) => {
            item[itemKey] = new clazz(item[itemKey])
          })
        } else {
          item[schemaKey] = new clazz(item[schemaKey])
        }
      } else {
        if (schemaKey === "*") {
          Object.values(item).forEach((itemValue) => ensureTypes(schemaValue, itemValue))
        } else {
          ensureTypes(schemaValue,  item[schemaKey])
        }
      }
    })
  }

  ensureTypes(itemSchema, item)

  return item
}

/**
 * Fetch the state from the persistence adapter
 * @return {Promise<void>}
 */
const rehydrateState = async function rehydrateState() {
  if (this.rehydrate && this.lastRehydrate === null) {
    try {
      const transactions = (this.adapter && hasMethod(this.adapter, 'getItem'))
        ? (rehydrateItem(await this.adapter.getItem('transactions'), 'transactions') || this.store.transactions)
        : this.store.transactions;
      const wallets = (this.adapter && hasMethod(this.adapter, 'getItem'))
        ? (rehydrateItem(await this.adapter.getItem('wallets'), 'wallets') || this.store.wallets)
        : this.store.wallets;
      const chains = (this.adapter && hasMethod(this.adapter, 'getItem'))
        ? (rehydrateItem(await this.adapter.getItem('chains'), 'chains') || this.store.chains)
        : this.store.chains;
      const instantLocks = (this.adapter && hasMethod(this.adapter, 'getItem'))
        ? (rehydrateItem(await this.adapter.getItem('instantLocks'), 'instantLocks') || this.store.instantLocks)
        : this.store.instantLocks;

      this.store.transactions = mergeHelper(this.store.transactions, transactions);
      this.store.wallets = mergeHelper(this.store.wallets, wallets);
      this.store.chains = mergeHelper(this.store.chains, chains);
      this.store.instantLocks = mergeHelper(this.store.instantLocks, instantLocks);
      this.lastRehydrate = +new Date();
      this.emit(REHYDRATE_STATE_SUCCESS, { type: REHYDRATE_STATE_SUCCESS, payload: null });
    } catch (e) {
      this.emit(REHYDRATE_STATE_FAILED, { type: REHYDRATE_STATE_FAILED, payload: e });
      throw e;
    }
  }
};
module.exports = rehydrateState;
