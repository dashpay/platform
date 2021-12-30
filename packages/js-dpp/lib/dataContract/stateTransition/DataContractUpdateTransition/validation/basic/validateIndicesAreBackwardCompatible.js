const lodashGet = require('lodash.get');

const DataContractHaveNewUniqueIndexError = require('../../../../../errors/consensus/basic/dataContract/DataContractHaveNewUniqueIndexError');
const DataContractUniqueIndicesChangedError = require("../../../../../errors/consensus/basic/dataContract/DataContractUniqueIndicesChangedError");
const DataContractInvalidIndexDefinitionUpdateError = require("../../../../../errors/consensus/basic/dataContract/DataContractInvalidIndexDefinitionUpdateError");

const getPropertyDefinitionByPath = require('../../../../getPropertyDefinitionByPath');

const serializer = require('../../../../../util/serializer');

const ValidationResult = require('../../../../../validation/ValidationResult');

/**
 * Get one of old unique indices that have been changed
 *
 * @param {Object<string, Object>} nameIndexMap
 * @param {string} documentType
 * @param {Object} existingSchema
 *
 * @returns {object|undefined}
 */
function getChangedOldUniqueIndex(nameIndexMap, documentType, existingSchema) {
  // Checking every unique existing index if it has been altered
  return (existingSchema.indices || []).find(
    (indexDefinition) => (
      !serializer.encode(indexDefinition).equals(
        serializer.encode(nameIndexMap[indexDefinition.name]),
      ) && indexDefinition.unique === true
    ),
  );
}

/**
 * Get one of the old non-unique indices that have been wrongly changed
 *
 * @param {Object<string, Object>} nameIndexMap
 * @param {string} documentType
 * @param {object} existingSchema
 *
 * @returns {object}
 */
function getWronglyUpdatedNonUniqueIndex(nameIndexMap, documentType, existingSchema) {
  // Checking every existing non-unique index, and it's respective new index
  // if they are changed per spec
  return (existingSchema.indices || []).find((indexDefinition) => {
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
    ).find((propertyWithOrder) => {
      const propertyName = Object.keys(propertyWithOrder)[0];

      return Boolean(
        getPropertyDefinitionByPath(existingSchema, propertyName),
      );
    });

    return notNewProperty !== undefined;
  });
}

/**
 * Get one of the new indices that have unique flag
 *
 * @param {string} documentType
 * @param {object} existingSchema
 * @param {object} newSchema
 *
 * @returns {object}
 */
function getNewUniqueIndex(documentType, existingSchema, newSchema) {
  const newSchemaIndices = lodashGet(newSchema, `${documentType}.indices`);

  const existingIndexNames = (existingSchema.indices || []).map(
    (indexDefinition) => indexDefinition.name,
  );

  // Gather only newly defined indices
  const newIndices = (newSchemaIndices || []).filter(
    (indexDefinition) => !existingIndexNames.includes(indexDefinition.name),
  );

  return (newIndices || []).find((indexDefinition) => indexDefinition.unique === true);
}

/**
 * Get one of the new indices that have old properties in them in the wrong order
 *
 * @param {string} documentType
 * @param {object} existingSchema
 * @param {object[]} newDocumentDefinitions
 *
 * @returns {object}
 */
function getWronglyConstructedNewIndex(documentType, existingSchema, newDocumentDefinitions) {
  const newSchemaIndices = lodashGet(newDocumentDefinitions, `${documentType}.indices`);

  const existingIndexNames = (existingSchema.indices || []).map(
    (indexDefinition) => indexDefinition.name,
  );

  const existingIndexedProperties = new Set((existingSchema.indices || []).reduce(
    (properties, indexDefinition) => [
      ...properties,
      ...indexDefinition.properties.map((definition) => Object.keys(definition)[0]),
    ], [],
  ));

  // Build an index of all possible allowed combinations
  // of old indices to check later
  const existingIndexSnapshots = (existingSchema.indices || []).reduce(
    (snapshots, indexDefinition) => [
      ...snapshots,
      ...indexDefinition.properties.map((_, index) => (
        serializer.encode(indexDefinition.properties.slice(0, index + 1)).toString('hex')
      )),
    ], [],
  );

  // Gather only newly defined indices
  const newIndices = (newSchemaIndices || []).filter(
    (indexDefinition) => !existingIndexNames.includes(indexDefinition.name),
  );

  return (newIndices || []).find((indexDefinition) => {
    const existingProperties = indexDefinition.properties.filter(
      (prop) => existingIndexedProperties.has(Object.keys(prop)[0]),
    );

    // if no old properties being used - skip
    if (existingProperties.length === 0) {
      return false;
    }

    // build a partial snapshot of the new index
    // containing only first part of it with a
    // length equal to number of old properties used
    // sine they should be in the beginning of the
    // index we can check it with previously built snapshot combinations
    const partialNewIndexSnapshot = serializer.encode(
      indexDefinition.properties.slice(0, existingProperties.length),
    ).toString('hex');

    return !existingIndexSnapshots.includes(partialNewIndexSnapshot);
  });
}

/**
 * Validate indices have not been changed
 *
 * @param {Object} existingDocumentDefinitions
 * @param {Object} newDocumentDefinitions
 *
 * @returns {ValidationResult}
 */
function validateIndicesAreBackwardCompatible(existingDocumentDefinitions, newDocumentDefinitions) {
  const result = new ValidationResult();

  Object.entries(existingDocumentDefinitions)
    .find(([documentType, existingSchema]) => {
      // Building name - index map for easier search
      const nameIndexMap = (newDocumentDefinitions[documentType].indices || [])
        .reduce((map, indexDefinition) => ({
          ...map,
          [indexDefinition.name]: indexDefinition,
        }), {});

      const changedUniqueExistingIndex = getChangedOldUniqueIndex(
        nameIndexMap,
        documentType,
        existingSchema,
      );

      if (changedUniqueExistingIndex !== undefined) {
        result.addError(
          new DataContractUniqueIndicesChangedError(
            documentType, changedUniqueExistingIndex.name,
          ),
        );

        return true;
      }

      const wronglyUpdatedIndex = getWronglyUpdatedNonUniqueIndex(
        nameIndexMap,
        documentType,
        existingSchema,
      );

      if (wronglyUpdatedIndex !== undefined) {
        result.addError(
          new DataContractInvalidIndexDefinitionUpdateError(
            documentType, wronglyUpdatedIndex.name,
          ),
        );

        return true;
      }

      const newUniqueIndex = getNewUniqueIndex(
        documentType, existingSchema, newDocumentDefinitions,
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
        documentType, existingSchema, newDocumentDefinitions,
      );

      if (wronglyConstructedNewIndex !== undefined) {
        result.addError(
          new DataContractInvalidIndexDefinitionUpdateError(
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
