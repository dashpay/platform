const lodashGet = require('lodash.get');
const DataContractHaveNewIndexWithOldPropertiesError = require('../../../../../errors/consensus/basic/dataContract/DataContractHaveNewIndexWithOldPropertiesError');
const DataContractHaveNewUniqueIndexError = require('../../../../../errors/consensus/basic/dataContract/DataContractHaveNewUniqueIndexError');
const DataContractIndicesChangedError = require('../../../../../errors/consensus/basic/dataContract/DataContractIndicesChangedError');
const DataContractNonUniqueIndexUpdateError = require('../../../../../errors/consensus/basic/dataContract/DataContractNonUniqueIndexUpdateError');

const serializer = require('../../../../../util/serializer');

const ValidationResult = require('../../../../../validation/ValidationResult');

/**
 * Get one of old unique indices that have been changed
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object|undefined}
 */
function getChangedOldUniqueIndex(documentType, oldSchema, newDocuments) {
  const path = `${documentType}.indices`;

  const newSchemaIndices = lodashGet(newDocuments, path);

  // Building name - index map for easier search
  const nameIndexMap = (newSchemaIndices || []).reduce((map, indexDefinition) => ({
    ...map,
    [indexDefinition.name]: indexDefinition,
  }), {});

  // Checking every unique old and it's respective new index
  // if they are have the same definition
  const changedIndex = (oldSchema.indices || []).find(
    (indexDefinition) => (
      !serializer.encode(indexDefinition).equals(
        serializer.encode(nameIndexMap[indexDefinition.name]),
      ) && indexDefinition.unique === true
    ),
  );

  return changedIndex;
}

/**
 * Get one of the old non-unique indices that have been wrongly changed
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object}
 */
function getWronglyUpdatedNonUniqueIndex(documentType, oldSchema, newDocuments) {
  const path = `${documentType}.indices`;

  const newSchemaIndices = lodashGet(newDocuments, path);

  const oldPropertyNames = Object.keys(oldSchema.properties);

  // Building name - index map for easier search
  const nameIndexMap = (newSchemaIndices || []).reduce((map, indexDefinition) => ({
    ...map,
    [indexDefinition.name]: indexDefinition,
  }), {});

  // Checking every old non-unique index and it's respective new index
  // if they are have changes per spec
  const changedIndex = (oldSchema.indices || []).find(
    (indexDefinition) => {
      if (indexDefinition.unique === true) {
        return false;
      }

      const newIndexDefinition = nameIndexMap[indexDefinition.name];

      // creating new index definition snapshot
      // with the same amount of properties as old one
      // if they are the same and in the same order
      // later check will return nothing
      const newIndexSnapshot = {
        name: indexDefinition.name,
        properties: newIndexDefinition.properties.slice(
          0, indexDefinition.properties.length,
        ),
      };

      if (!serializer.encode(indexDefinition).equals(
        serializer.encode(newIndexSnapshot),
      )) {
        return true;
      }

      // check that rest of the properties are newly defined ones
      const notNewProperty = newIndexDefinition.properties.slice(
        indexDefinition.properties.length,
      ).find((item) => oldPropertyNames.includes(Object.keys(item)[0]));

      if (notNewProperty !== undefined) {
        return true;
      }

      return false;
    },
  );

  return changedIndex;
}

/**
 * Get one of the new indices that have unique flag
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object}
 */
function getNewUniqueIndex(documentType, oldSchema, newDocuments) {
  const newSchemaIndices = lodashGet(newDocuments, `${documentType}.indices`);

  const oldIndexNames = (oldSchema.indices || []).map(
    (indexDefinition) => indexDefinition.name,
  );

  // Gather only newly defined indices
  const newIndices = (newSchemaIndices || []).filter(
    (indexDefinition) => !oldIndexNames.includes(indexDefinition.name),
  );

  const indexUnique = (newIndices || []).find((indexDefinition) => indexDefinition.unique === true);

  return indexUnique;
}

/**
 * Get one of the new indices that have old properties in them in the wrong order
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object}
 */
function getWronglyConstructedNewIndex(documentType, oldSchema, newDocuments) {
  const newSchemaIndices = lodashGet(newDocuments, `${documentType}.indices`);

  const oldIndexNames = (oldSchema.indices || []).map(
    (indexDefinition) => indexDefinition.name,
  );

  const oldIndexedProperties = new Set((oldSchema.indices || []).reduce(
    (properties, indexDefinition) => [
      ...properties,
      ...indexDefinition.properties.map((definition) => Object.keys(definition)[0]),
    ], [],
  ));

  // Build an index of all possible allowed combinations
  // of old indices to check later
  const oldIndexSnapshots = (oldSchema.indices || []).reduce(
    (snapshots, indexDefinition) => [
      ...snapshots,
      ...indexDefinition.properties.map((_, index) => (
        serializer.encode(indexDefinition.properties.slice(0, index + 1)).toString('hex')
      )),
    ], [],
  );

  // Gather only newly defined indices
  const newIndices = (newSchemaIndices || []).filter(
    (indexDefinition) => !oldIndexNames.includes(indexDefinition.name),
  );

  const wronglyContstructedIndex = (newIndices || []).find((indexDefinition) => {
    const oldProperties = indexDefinition.properties.filter(
      (prop) => oldIndexedProperties.has(Object.keys(prop)[0]),
    );

    // if no old properties being used - skip
    if (oldProperties.length === 0) {
      return false;
    }

    // build a partial snapshot of the new index
    // containing only first part of it with a
    // length equal to number of old properties used
    // sine they should be in the beginning of the
    // index we can check it with previously built snapshot combinations
    const partialNewIndexSnapshot = serializer.encode(
      indexDefinition.properties.slice(0, oldProperties.length),
    ).toString('hex');

    return !oldIndexSnapshots.includes(partialNewIndexSnapshot);
  });

  return wronglyContstructedIndex;
}

/**
 * Validate indices have not been changed
 *
 * @param {Object} oldDocuments
 * @param {Object} newDocuments
 *
 * @returns {ValidationResult}
 */
function validateIndicesAreBackwardCompatible(oldDocuments, newDocuments) {
  const result = new ValidationResult();

  Object.entries(oldDocuments)
    .find(([documentType, oldSchema]) => {
      const changedOldIndex = getChangedOldUniqueIndex(
        documentType, oldSchema, newDocuments,
      );

      if (changedOldIndex !== undefined) {
        result.addError(
          new DataContractIndicesChangedError(
            documentType, changedOldIndex.name,
          ),
        );

        return true;
      }

      const wronglyUpdatedIndex = getWronglyUpdatedNonUniqueIndex(
        documentType, oldSchema, newDocuments,
      );

      if (wronglyUpdatedIndex !== undefined) {
        result.addError(
          new DataContractNonUniqueIndexUpdateError(
            documentType, wronglyUpdatedIndex.name,
          ),
        );

        return true;
      }

      const newUniqueIndex = getNewUniqueIndex(
        documentType, oldSchema, newDocuments,
      );

      if (newUniqueIndex !== undefined) {
        result.addError(
          new DataContractHaveNewUniqueIndexError(
            documentType, newUniqueIndex.name,
          ),
        );

        return true;
      }

      const wronglyConstructedNewIndex = getWronglyConstructedNewIndex(
        documentType, oldSchema, newDocuments,
      );

      if (wronglyConstructedNewIndex !== undefined) {
        result.addError(
          new DataContractHaveNewIndexWithOldPropertiesError(
            documentType, wronglyConstructedNewIndex.name,
          ),
        );

        return true;
      }

      return false;
    });

  return result;
}

module.exports = validateIndicesAreBackwardCompatible;
