const lodashGet = require('lodash.get');
const DataContractIndicesChangedError = require('../../../../../errors/consensus/basic/dataContract/DataContractIndicesChangedError');

const serializer = require('../../../../../util/serializer');

const ValidationResult = require('../../../../../validation/ValidationResult');

/**
 * Validate indices have not been changed
 *
 * @param {Object} oldDocuments
 * @param {Object} newDocuments
 *
 * @returns {ValidationResult}
 */
function validateIndicesAreNotChanged(oldDocuments, newDocuments) {
  const result = new ValidationResult();

  // Check that old index dinfitions are intact
  let changedDocumentEntry = Object.entries(oldDocuments)
    .find(([documentType, oldSchema]) => {
      const path = `${documentType}.indices`;

      const newSchemaIndices = lodashGet(newDocuments, path);

      const nameIndexMap = (newSchemaIndices || []).reduce((map, indexDefinition) => ({
        ...map,
        [indexDefinition.name]: indexDefinition,
      }), {});

      const changedIndices = (oldSchema.indices || []).find(
        (indexDefinition) => (
          !serializer.encode(indexDefinition).equals(
            serializer.encode(nameIndexMap[indexDefinition.name]),
          )
        ),
      );

      return changedIndices !== undefined;
    });

  let [invalidDocumentType] = changedDocumentEntry || [];

  if (invalidDocumentType) {
    result.addError(new DataContractIndicesChangedError(invalidDocumentType));

    return result;
  }

  // Check that new indices are about new properties only
  changedDocumentEntry = Object.entries(oldDocuments)
    .find(([documentType, oldSchema]) => {
      const newSchemaIndices = lodashGet(newDocuments, `${documentType}.indices`);

      const oldIndexNames = (oldSchema.indices || []).map(
        (indexDefinition) => indexDefinition.name,
      );
      const oldPropertyNames = Object.keys(oldSchema.properties);

      const newIndices = (newSchemaIndices || []).filter(
        (indexDefinition) => !oldIndexNames.includes(indexDefinition.name),
      );

      const indexContainingOldPropertiesOrUnique = (newIndices || []).find((indexDefinition) => {
        if (indexDefinition.unique === true) {
          return true;
        }

        const propertyNames = indexDefinition.properties.reduce((list, propertyObj) => {
          const keys = Object.keys(propertyObj).filter((n) => !n.startsWith('$'));
          return [
            ...list,
            ...keys,
          ];
        }, []);

        return propertyNames.find((n) => oldPropertyNames.includes(n)) !== undefined;
      });

      return indexContainingOldPropertiesOrUnique !== undefined;
    });

  ([invalidDocumentType] = changedDocumentEntry || []);

  if (invalidDocumentType) {
    result.addError(new DataContractIndicesChangedError(invalidDocumentType));
  }

  return result;
}

module.exports = validateIndicesAreNotChanged;
