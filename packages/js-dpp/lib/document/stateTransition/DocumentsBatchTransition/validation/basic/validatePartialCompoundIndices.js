const lodashGet = require('lodash.get');

const ValidationResult = require('../../../../../validation/ValidationResult');

const InconsistentCompoundIndexDataError = require('../../../../../errors/InconsistentCompoundIndexDataError');

/**
 * @typedef validatePartialCompoundIndices
 * @param {Buffer} ownerId
 * @param {Array<
 *         RawDocumentCreateTransition|
 *         RawDocumentReplaceTransition
 *         >} rawDocumentTransitions
 * @param {DataContract} dataContract
 * @return {ValidationResult}
 */
function validatePartialCompoundIndices(
  ownerId,
  rawDocumentTransitions,
  dataContract,
) {
  const result = new ValidationResult();

  rawDocumentTransitions.forEach((rawDocumentTransition) => {
    const documentSchema = dataContract.getDocumentSchema(rawDocumentTransition.$type);

    if (!documentSchema.indices) {
      return;
    }

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

          return lodashGet(rawDocumentTransition, propertyPath);
        });

        const allAreDefined = data.every((item) => item !== undefined);
        const allAreUndefined = data.every((item) => item === undefined);

        const isOk = allAreDefined || allAreUndefined;

        if (!isOk) {
          result.addError(new InconsistentCompoundIndexDataError(
            rawDocumentTransition.$type,
            indexDefinition,
          ));
        }
      });
  });

  return result;
}

module.exports = validatePartialCompoundIndices;
