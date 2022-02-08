const systemIndices = [
  {
    properties: [{ $id: 'asc' }],
    unique: true,
  },
];

/**
 * @typedef {findAppropriateIndex}
 * @param {Array[]} whereClauses
 * @param {Object} documentSchema
 * @return {Object}
 */
function findAppropriateIndex(whereClauses, documentSchema) {
  const documentIndices = (documentSchema.indices || []).concat(systemIndices);

  const whereProperties = (whereClauses || []).map(([propertyName]) => propertyName);

  const uniqueWhereProperties = [...new Set(whereProperties)];

  return documentIndices.find((indexDefinition) => {
    const indexedProperties = indexDefinition.properties
      .map((indexedProperty) => Object.keys(indexedProperty)[0]);

    return !uniqueWhereProperties.find((propertyName) => !indexedProperties.includes(propertyName));
  });
}

module.exports = findAppropriateIndex;
