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
  const hasChangedArray = await Promise.all(
    Object.entries(oldDocuments)
      .filter(([, schema]) => schema.indices !== undefined)
      .map(
        async ([property, oldSchema]) => {
          const path = `${property}.indices`;

          const newSchemaIndices = lodashGet(newDocuments, path);

          return !serializer.encode(oldSchema.indices).equals(serializer.encode(newSchemaIndices));
        },
      ),
  );

  const hasChanged = hasChangedArray.reduce(
    (nextItem, accumulator) => accumulator || nextItem,
    false,
  );

  // check there are no document definition with indices were added
  const oldDocumentDefinitionNames = Object.keys(oldDocuments);
  const addedDocumentDefinitions = Object.entries(newDocuments).filter(
    ([documentName]) => !oldDocumentDefinitionNames.includes(documentName),
  );

  const thereAreNewIndexDefinitions = addedDocumentDefinitions
    .filter(([, schema]) => schema.indices !== undefined)
    .length > 0;

  if (hasChanged || thereAreNewIndexDefinitions) {
    result.addError(new DataContractIndicesChangedError());
  }

  return result;
}

module.exports = validateIndicesAreNotChanged;
