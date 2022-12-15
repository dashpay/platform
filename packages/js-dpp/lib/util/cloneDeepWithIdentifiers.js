const lodashCloneDeepWith = require('lodash/cloneDeepWith');
const Identifier = require('../identifier/Identifier');

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
