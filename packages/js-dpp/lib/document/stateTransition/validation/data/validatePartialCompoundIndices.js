const ValidationResult = require('../../../../validation/ValidationResult');

const InconsistentCompoundIndexDataError = require('../../../../errors/InconsistentCompoundIndexDataError');

/**
 * @typedef validatePartialCompoundIndices
 * @param {string} ownerId
 * @param {DocumentCreateTransition[]
 *         |DocumentReplaceTransition[]} documentTransitions
 * @param {DataContract} dataContract
 * @return {ValidationResult}
 */
function validatePartialCompoundIndices(
  ownerId,
  documentTransitions,
  dataContract,
) {
  const result = new ValidationResult();

  documentTransitions.forEach((documentTransition) => {
    const documentSchema = dataContract.getDocumentSchema(documentTransition.getType());

    if (!documentSchema.indices) {
      return;
    }

    const rawDocumentTransition = documentTransition.toObject();

    documentSchema.indices
      .filter((index) => index.unique && index.properties.length > 1)
      .forEach((indexDefinition) => {
        const data = indexDefinition.properties.map((property) => {
          const [propertyPath] = Object.keys(property);

          if (propertyPath === '$ownerId') {
            return ownerId;
          }

          if (propertyPath.startsWith('$')) {
            return rawDocumentTransition[propertyPath];
          }

          return documentTransition.get(propertyPath);
        });

        const allAreDefined = data.every((item) => item !== undefined);
        const allAreUndefined = data.every((item) => item === undefined);

        const isOk = allAreDefined || allAreUndefined;

        if (!isOk) {
          result.addError(new InconsistentCompoundIndexDataError(
            documentTransition.getType(),
            indexDefinition,
          ));
        }
      });
  });

  return result;
}

module.exports = validatePartialCompoundIndices;
