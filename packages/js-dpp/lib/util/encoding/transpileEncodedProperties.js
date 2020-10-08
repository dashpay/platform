const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const cloneDeepRawData = require('../cloneDeepRawData');

const EncodedBuffer = require('./EncodedBuffer');

/**
 *
 * @param {DataContract} dataContract
 * @param {string} type
 * @param {Object} originalObject
 * @param {function} transpileFunction
 * @return {Object}
 */
function transpileEncodedProperties(dataContract, type, originalObject, transpileFunction) {
  const clonedObject = cloneDeepRawData(originalObject);

  const encodedProperties = dataContract.getBinaryProperties(type);

  Object.keys(encodedProperties)
    .forEach((propertyPath) => {
      const value = lodashGet(clonedObject, propertyPath);
      if (value !== undefined) {
        lodashSet(
          clonedObject,
          propertyPath,
          transpileFunction(value, EncodedBuffer.ENCODING.BASE64, propertyPath),
        );
      }
    });

  return clonedObject;
}

module.exports = transpileEncodedProperties;
