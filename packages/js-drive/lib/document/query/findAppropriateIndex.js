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
 * @return {Object|undefined}
 */
function findAppropriateIndex(whereClauses, documentSchema) {
  const documentIndices = (documentSchema.indices || []).concat(systemIndices);

  const whereProperties = (whereClauses || []).map(([propertyName]) => propertyName);

  const uniqueWhereProperties = [...new Set(whereProperties)];

  return documentIndices.find((indexDefinition) => {
    const indexedProperties = indexDefinition.properties
      .map((indexedProperty) => Object.keys(indexedProperty)[0]);

    const indexPlaces = uniqueWhereProperties
      .map((propertyName) => indexedProperties.indexOf(propertyName))
      .sort();

    const correctIndexPlaces = Array(indexPlaces.length).fill().map((v, i) => i);

    return indexPlaces.length <= uniqueWhereProperties.length
      && indexPlaces.every((item, i) => item === correctIndexPlaces[i]);
  });
}

module.exports = findAppropriateIndex;
