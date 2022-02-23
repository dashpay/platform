const cloneDeep = require('lodash.clonedeep');

/**
 * @typedef {sortWhereClausesAccordingToIndex}
 * @param {[string, string, *][]} whereClauses
 * @param {Object} indexDefinition
 *
 * @returns {[string, string, *][]}
 */
function sortWhereClausesAccordingToIndex(whereClauses, indexDefinition) {
  const indexedProperties = indexDefinition.properties
    .map((indexedProperty) => Object.keys(indexedProperty)[0]);

  const clonedWhereClauses = cloneDeep(whereClauses);

  clonedWhereClauses.sort((a, b) => (
    indexedProperties.indexOf(a[0]) - indexedProperties.indexOf(b[0])
  ));

  return clonedWhereClauses;
}

module.exports = sortWhereClausesAccordingToIndex;
