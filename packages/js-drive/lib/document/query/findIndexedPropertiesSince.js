/**
 * Find all lists of indexed properties of specific length since selected one
 *
 * @typedef findIndexedPropertiesSince
 *
 * @param {string} property
 * @param {number} numberOfProperties
 * @param {Object} documentSchema
 *
 * @return {string[][]}
 */
function findIndexedPropertiesSince(property, numberOfProperties, documentSchema) {
  const documentIndices = (documentSchema.indices || []);

  const documentIndexedPropertiesList = documentIndices.map((indices) => (
    indices.properties.map((prop) => Object.keys(prop)[0])
  ));

  return documentIndexedPropertiesList
    .map((propertyList) => {
      if (propertyList.includes(property)) {
        const indexOfProperty = propertyList.indexOf(property);

        return propertyList.slice(indexOfProperty, indexOfProperty + numberOfProperties);
      }

      return [];
    })
    .filter((propertyList) => propertyList.length === numberOfProperties);
}

module.exports = findIndexedPropertiesSince;
