const { merge } = require('lodash');
const { InstantLock, Transaction, BlockHeader } = require('@dashevo/dashcore-lib');
const { hasMethod } = require('../../../utils');

const mergeHelper = (initial = {}, additional = {}) => merge(initial, additional);
const { REHYDRATE_STATE_FAILED, REHYDRATE_STATE_SUCCESS } = require('../../../EVENTS');
const logger = require('../../../logger');

const storeTypesSchema = {
  transactions: {
    '*': Transaction,
  },
  instantLocks: {
    '*': InstantLock,
  },
  wallets: {
    '*': {
      addresses: {
        external: {
          '*': {
            utxos: {
              // eslint-disable-next-line func-names
              '*': function (item) {
                // TODO: resolve type inconsistency for address utxos
                try {
                  return new Transaction.UnspentOutput(item);
                } catch (e) {
                  return new Transaction.Output(item);
                }
              },
            },
          },
        },
        internal: {
          '*': {
            utxos: {
              // eslint-disable-next-line func-names
              '*': function (item) {
                // TODO: resolve type inconsistency for address utxos
                try {
                  return new Transaction.UnspentOutput(item);
                } catch (e) {
                  return new Transaction.Output(item);
                }
              },
            },
          },
        },
      },
    },
  },
  chains: {
    '*': {
      blockHeaders: {
        '*': BlockHeader,
      },
    },
  },
};

const castItemTypes = (item, schema) => {
  Object.entries(schema).forEach(([schemaKey, schemaValue]) => {
    if (schemaValue.constructor.name !== 'Object') {
      const Clazz = schemaValue;
      if (schemaKey === '*') {
        Object.keys(item).forEach((itemKey) => {
          if (!item[itemKey]) {
            throw new Error(`No item key "${itemKey}" found for item ${JSON.stringify(item)}`);
          }

          // eslint-disable-next-line no-param-reassign
          item[itemKey] = new Clazz(item[itemKey]);
        });
      } else {
        if (!item[schemaKey]) {
          throw new Error(`No item key "${schemaKey}" found for item ${JSON.stringify(item)}`);
        }

        // eslint-disable-next-line no-param-reassign
        item[schemaKey] = new Clazz(item[schemaKey]);
      }
    } else if (schemaKey === '*') {
      Object.values(item).forEach((itemValue) => castItemTypes(itemValue, schemaValue));
    } else {
      castItemTypes(item[schemaKey], schemaValue);
    }
  });

  return item;
};

/**
 * Fetch the state from the persistence adapter
 * @return {Promise<void>}
 */
const rehydrateState = async function rehydrateState() {
  if (this.rehydrate && this.lastRehydrate === null) {
    try {
      const storeItems = {
        transactions: null,
        wallets: null,
        chains: null,
        instantLocks: null,
      };
      const keys = Object.keys(storeItems);
      // Obtain items from storage adapter
      for (let i = 0; i < keys.length; i += 1) {
        const itemKey = keys[i];

        let item;
        if (this.adapter && hasMethod(this.adapter, 'getItem')) {
          // eslint-disable-next-line no-await-in-loop
          item = await this.adapter.getItem(itemKey);
        }

        storeItems[itemKey] = item || this.store[itemKey];
      }

      // Cast store items to correct data types
      try {
        castItemTypes(storeItems, storeTypesSchema);
      } catch (e) {
        logger.error('Error casting storage items types: possibly data schema mismatch');
        throw e;
      }

      // Merge with the current items in store
      Object.keys(storeItems).forEach((itemKey) => {
        this.store[itemKey] = mergeHelper(this.store[itemKey], storeItems[itemKey]);
      });

      this.lastRehydrate = +new Date();
      this.emit(REHYDRATE_STATE_SUCCESS, { type: REHYDRATE_STATE_SUCCESS, payload: null });
    } catch (e) {
      this.emit(REHYDRATE_STATE_FAILED, { type: REHYDRATE_STATE_FAILED, payload: e });
      throw e;
    }
  }
};
module.exports = rehydrateState;
