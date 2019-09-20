const { cloneDeep } = require('lodash');
/**
 * Return the content of the store
 * @return {{} & any}
 */
module.exports = function getStore() {
  return cloneDeep(this.store);
};
