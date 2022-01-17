const isEqual = require('lodash.isequal');

const systemIndices = [
  {
    properties: [{ $id: 'asc' }],
    unique: true,
  },
];

/**
 * @typedef {findAppropriateIndex}
 * @param {Object[]} whereClauses
 * @param {Object} documentSchema
 * @return {Object}
 */
function findAppropriateIndex(whereClauses, documentSchema) {
  const documentIndices = (documentSchema.indices || []).concat(systemIndices);

  const whereProperties = (whereClauses || []).map(([field]) => field);

  const uniqueWhereProperties = [...new Set(whereProperties)];

  return documentIndices.find((indexDefinition) => {
    const indexedProperties = indexDefinition.properties
      .map((indexedProperty) => Object.keys(indexedProperty)[0]);

    return isEqual(indexedProperties, uniqueWhereProperties);
  });
}

module.exports = findAppropriateIndex;
