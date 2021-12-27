const lodashGet = require('lodash.get');
const DataContractHaveNewIndexWithOldPropertiesError = require('../../../../../errors/consensus/basic/dataContract/DataContractHaveNewIndexWithOldPropertiesError');
const DataContractHaveNewUniqueIndexError = require('../../../../../errors/consensus/basic/dataContract/DataContractHaveNewUniqueIndexError');
const DataContractIndicesChangedError = require('../../../../../errors/consensus/basic/dataContract/DataContractIndicesChangedError');
const DataContractNonUniqueIndexUpdateError = require('../../../../../errors/consensus/basic/dataContract/DataContractNonUniqueIndexUpdateError');

const serializer = require('../../../../../util/serializer');

const ValidationResult = require('../../../../../validation/ValidationResult');

/**
 * Check one of old unique indices have been changed
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object}
 */
function checkOldIndicesChanged(documentType, oldSchema, newDocuments) {
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
 * Check if some of the old non-unique indices have chaned per spec
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object}
 */
function checkNonUniqueIndicesChangedPerSpec(documentType, oldSchema, newDocuments) {
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
 * Check if some of the new indices have unique flag
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object}
 */
function checkNewIndicesAreUnique(documentType, oldSchema, newDocuments) {
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
 * Check if some of the new indices have old properties
 *
 * @param {string} documentType
 * @param {object} oldSchema
 * @param {object[]} newDocuments
 *
 * @returns {object}
 */
function checkNewIndicesHaveOldProperties(documentType, oldSchema, newDocuments) {
  const newSchemaIndices = lodashGet(newDocuments, `${documentType}.indices`);

  const oldIndexNames = (oldSchema.indices || []).map(
    (indexDefinition) => indexDefinition.name,
  );
  const oldPropertyNames = Object.keys(oldSchema.properties);

  // Gather only newly defined indices
  const newIndices = (newSchemaIndices || []).filter(
    (indexDefinition) => !oldIndexNames.includes(indexDefinition.name),
  );

  const indexHaveOldProperties = (newIndices || []).find((indexDefinition) => {
    const propertyNames = indexDefinition.properties.reduce((list, propertyObj) => {
      const keys = Object.keys(propertyObj).filter((n) => !n.startsWith('$'));
      return [
        ...list,
        ...keys,
      ];
    }, []);

    return propertyNames.find((n) => oldPropertyNames.includes(n)) !== undefined;
  });

  return indexHaveOldProperties;
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
      const changedIndex = checkOldIndicesChanged(
        documentType, oldSchema, newDocuments,
      );

      if (changedIndex !== undefined) {
        result.addError(
          new DataContractIndicesChangedError(
            documentType, changedIndex.name,
          ),
        );

        return true;
      }

      const nonSpecNonUniqueIndex = checkNonUniqueIndicesChangedPerSpec(
        documentType, oldSchema, newDocuments,
      );

      if (nonSpecNonUniqueIndex !== undefined) {
        result.addError(
          new DataContractNonUniqueIndexUpdateError(
            documentType, nonSpecNonUniqueIndex.name,
          ),
        );

        return true;
      }

      const newUniqueIndex = checkNewIndicesAreUnique(
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

      const newIndexWithOldProperties = checkNewIndicesHaveOldProperties(
        documentType, oldSchema, newDocuments,
      );

      if (newIndexWithOldProperties !== undefined) {
        result.addError(
          new DataContractHaveNewIndexWithOldPropertiesError(
            documentType, newIndexWithOldProperties.name,
          ),
        );

        return true;
      }

      return false;
    });

  return result;
}

module.exports = validateIndicesAreBackwardCompatible;
