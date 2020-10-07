const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');
const cloneDeepRawData = require('../cloneDeepRawData');
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

  const encodedProperties = dataContract.getEncodedProperties(type);

  Object.keys(encodedProperties)
    .forEach((propertyPath) => {
      const property = encodedProperties[propertyPath];

      if (property.contentEncoding) {
        const value = lodashGet(clonedObject, propertyPath);
        if (value !== undefined) {
          lodashSet(
            clonedObject,
            propertyPath,
            transpileFunction(value, property.contentEncoding, propertyPath),
          );
        }
      }
    });

  return clonedObject;
}

module.exports = transpileEncodedProperties;
