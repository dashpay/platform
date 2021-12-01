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
 * @returns {Promise<ValidationResult>}
 */
async function validateIndicesAreNotChanged(oldDocuments, newDocuments) {
  const result = new ValidationResult();

  // Check that old index dinfitions are intact
  const changedDocumentEntry = Object.entries(oldDocuments)
    .find(([documentType, oldSchema]) => {
      const path = `${documentType}.indices`;

      const newSchemaIndices = lodashGet(newDocuments, path);

      return !serializer.encode(oldSchema.indices).equals(serializer.encode(newSchemaIndices));
    });

  // check there are no document definition with indices were added
  const oldDocumentDefinitionTypes = Object.keys(oldDocuments);
  const newDocumentWithIndicesEntry = Object.entries(newDocuments)
    .find(([documentType, schema]) => (
      !oldDocumentDefinitionTypes.includes(documentType) && schema.indices !== undefined
    ));

  const [documentType] = (changedDocumentEntry || newDocumentWithIndicesEntry) || [];

  if (documentType) {
    result.addError(new DataContractIndicesChangedError(documentType));
  }

  return result;
}

module.exports = validateIndicesAreNotChanged;
