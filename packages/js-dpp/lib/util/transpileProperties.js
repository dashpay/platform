const lodashGet = require('lodash.get');
const lodashSet = require('lodash.set');

const cloneDeepWithIdentifiers = require('./cloneDeepWithIdentifiers');

/**
 *
 * @param {Object} object
 * @param {Object} properties
 * @param {Function} transpileFunction
 * @return {Object}
 */
function transpileProperties(object, properties, transpileFunction) {
  const clonedObject = cloneDeepWithIdentifiers(object);

  Object.keys(properties)
    .forEach((propertyPath) => {
      const value = lodashGet(clonedObject, propertyPath);

      if (value !== undefined) {
        lodashSet(
          clonedObject,
          propertyPath,
          transpileFunction(value, propertyPath),
        );
      }
    });

  return clonedObject;
}

module.exports = transpileProperties;
