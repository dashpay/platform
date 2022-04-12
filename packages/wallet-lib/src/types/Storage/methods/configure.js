const { has } = require('lodash');
const configureAdapter = require('../_configureAdapter');
const getDefaultAdapter = require('../_getDefaultAdapter');
const { CONFIGURED } = require('../../../EVENTS');
const logger = require('../../../logger')
const WalletStore = require('../../WalletStore/WalletStore');
const ChainStore = require('../../ChainStore/ChainStore');

const CURRENT_VERSION = 1

const castItemTypes = (item, schema) => {
  Object.entries(schema).forEach(([schemaKey, schemaValue]) => {
    if (schemaValue.constructor.name !== 'Object') {
      const Clazz = schemaValue;
      if (schemaKey === '*') {
        Object.keys(item).forEach((itemKey) => {
          // eslint-disable-next-line no-param-reassign2
          item[itemKey] = new Clazz(item[itemKey]);
        });
      } else {
        if (!item[schemaKey]) {
          throw new Error(`No schema key "${schemaKey}" found for item ${JSON.stringify(item)}`);
        }

        // eslint-disable-next-line no-param-reassign
        if (!(['string', 'number'].includes(schemaValue))) {
          item[schemaKey] = new Clazz(item[schemaKey]);
        }
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
 * To be called after instantialization as it contains all the async logic / test of adapters
 * @param opts
 * @return {Promise<void>}
 */
module.exports = async function configure(opts = {}) {
  this.rehydrate = has(opts, 'rehydrate') ? opts.rehydrate : this.rehydrate;
  this.autosave = has(opts, 'autosave') ? opts.autosave : this.autosave;
  this.adapter = await configureAdapter((opts.adapter) ? opts.adapter : await getDefaultAdapter());

  const version = await this.adapter.getItem('version');
  const wallets = await this.adapter.getItem('wallets');
  const chains = await this.adapter.getItem('chains');

  if ((wallets || chains) && version !== CURRENT_VERSION) {
    logger.warn('Storage validation error: schema mismatch: unknown version')
    await this.adapter.setItem('wallets', null)
    await this.adapter.setItem('chains', null)

    await this.adapter.setItem('version', 1)
  }

  try {
    castItemTypes(wallets, WalletStore.SCHEMA)
    castItemTypes(chains, ChainStore.SCHEMA)
  } catch (e) {
    await this.adapter.setItem('wallets', null)
    await this.adapter.setItem('chains', null)
  }

  this.createWalletStore(opts.walletId);
  this.createChainStore(opts.network);

  if (this.rehydrate) {
    await this.rehydrateState();
  }

  if (this.autosave) {
    this.startWorker();
  }

  this.configured = true;
  this.emit(CONFIGURED, { type: CONFIGURED, payload: null });
};
