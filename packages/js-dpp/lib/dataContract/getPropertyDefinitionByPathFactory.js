/**
 * @returns {getPropertyDefinitionByPath}
 */
function getPropertyDefinitionByPathFactory() {
  /**
   * Get user property definition
   *
   * @typedef getPropertyDefinitionByPath
   * @param {Object} documentDefinition
   * @param {string} path
   *
   * @return {Object}
   */
  function getPropertyDefinitionByPath(documentDefinition, path) {
    const [currentSegment, ...rest] = path.split('.');

    const { [currentSegment]: propertyDefinition } = (documentDefinition.properties || {});

    // nothing found return nothing
    if (!propertyDefinition) {
      return undefined;
    }

    // if there is nothing to lookup for next
    // return currently found property definition
    if (rest.length === 0) {
      return propertyDefinition;
    }

    const { type } = propertyDefinition;

    if (type === 'array') {
      const { items: itemsDefinition } = propertyDefinition;

      if (itemsDefinition.type === 'object') {
        return getPropertyDefinitionByPath(itemsDefinition, rest.join('.'));
      }
    }

    if (type === 'object') {
      // rince and repeat
      return getPropertyDefinitionByPath(propertyDefinition, rest.join('.'));
    }

    // the `rest` is not empty
    // but definition is not an object nor array
    // nothing to lookup for
    return undefined;
  }

  return getPropertyDefinitionByPath;
}

module.exports = getPropertyDefinitionByPathFactory;
