const { cloneDeep } = require('lodash');
const initialStore = require('../initialStore');
/**
 * Clear all the store and save the cleared store to the persistance adapter
 * @return {Promise<boolean>}
 */
const clearAll = async function () {
  this.store = cloneDeep(initialStore);
  return this.saveState();
};
module.exports = clearAll;
