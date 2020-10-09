const lodashCloneDeepWith = require('lodash.clonedeepwith');
const Identifier = require('../Identifier');

/**
 * Clone data which contains Identifiers
 *
 * @param {*} value
 * @return {*}
 */
function cloneDeepWithIdentifiers(value) {
  // eslint-disable-next-line consistent-return
  return lodashCloneDeepWith(value, (item) => {
    if (item instanceof Identifier) {
      return new Identifier(item.toBuffer());
    }
  });
}

module.exports = cloneDeepWithIdentifiers;
