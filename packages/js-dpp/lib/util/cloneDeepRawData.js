const lodashCloneDeepWith = require('lodash.clonedeepwith');
const EncodedBuffer = require('./encoding/EncodedBuffer');

/**
 * Clone data which contains EncodedBuffer
 *
 * @param {*} value
 * @return {*}
 */
function cloneDeepRawData(value) {
  // eslint-disable-next-line consistent-return
  return lodashCloneDeepWith(value, (item) => {
    if (item instanceof EncodedBuffer) {
      return new EncodedBuffer(item.toBuffer(), item.getEncoding());
    }
  });
}

module.exports = cloneDeepRawData;
