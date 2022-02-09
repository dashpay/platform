/**
 * Find all threesomes and twosomes of indexed properties
 * relative to the specific property
 *
 * @typedef findThreesomeOfIndexedProperties
 *
 * @param {string} property
 * @param {Object} documentSchema
 *
 * @returns {string[][]}
 */
function findThreesomeOfIndexedProperties(property, documentSchema) {
  const documentIndices = (documentSchema.indices || []);

  return documentIndices.map((indices) => {
    const properties = indices.properties.map((props) => Object.keys(props)[0]);

    if (properties.includes(property)) {
      const propertyIndex = properties.indexOf(property);

      return properties.slice(propertyIndex, propertyIndex + 2);
    }

    return [];
  }).filter((item) => item.length > 0);
}

module.exports = findThreesomeOfIndexedProperties;
