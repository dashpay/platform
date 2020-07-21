const { cloneDeep } = require('lodash');
/**
 * Return the content of the store
 * @return {Storage.store}
 */
module.exports = function getStore() {
  return cloneDeep(this.store);
};
