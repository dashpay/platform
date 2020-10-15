const lodashCloneDeepWith = require('lodash.clonedeepwith');

/**
 * Clone data which contains Identifiers
 *
 * @param {*} value
 * @return {*}
 */
function convertBuffersToArrays(value) {
  // eslint-disable-next-line consistent-return
  return lodashCloneDeepWith(value, (item) => {
    if (item instanceof Buffer) {
      return [...item];
    }
  });
}

module.exports = convertBuffersToArrays;
