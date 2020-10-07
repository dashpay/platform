const lodashSet = require('lodash.set');
const lodashGet = require('lodash.get');

const cloneDeepRawData = require('../cloneDeepRawData');

const EncodedBuffer = require('./EncodedBuffer');

/**
 *
 * @param {*} object
 * @param {Object} encodedProperties
 * @returns {*}
 */
function encodeObjectProperties(object, encodedProperties) {
  if (typeof object !== 'object' || object === null) {
    return object;
  }

  const clonedObject = cloneDeepRawData(object);

  Object.keys(encodedProperties)
    .forEach((propertyPath) => {
      const property = encodedProperties[propertyPath];

      if (property.contentEncoding) {
        const value = lodashGet(clonedObject, propertyPath);

        if (Buffer.isBuffer(value)) {
          lodashSet(
            clonedObject,
            propertyPath,
            EncodedBuffer.from(value, property.contentEncoding).toString(),
          );
        }
      }
    });

  return clonedObject;
}

module.exports = encodeObjectProperties;
