const { cloneDeep } = require('lodash');
const initialStore = require('../initialStore.json');
/**
 * Clear all the store and save the cleared store to the persistence adapter
 * @return {Promise<boolean>}
 */
const clearAll = async function clearAll() {
  this.store = cloneDeep(initialStore);
  return this.saveState();
};
module.exports = clearAll;
