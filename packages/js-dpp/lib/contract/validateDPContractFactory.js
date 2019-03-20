const JsonSchemaValidator = require('../validation/JsonSchemaValidator');

const DPContract = require('./DPContract');

const DuplicateIndexError = require('../errors/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../errors/UndefinedIndexPropertyError');
const UniqueIndexMustHaveUserIdPrefixError = require('../errors/UniqueIndexMustHaveUserIdPrefixError');

/**
 * @param validator
 * @return {validateDPContract}
 */
module.exports = function validateDPContractFactory(validator) {
  /**
   * @typedef validateDPContract
   * @param {DPContract|Object} dpContract
   * @return {ValidationResult}
   */
  function validateDPContract(dpContract) {
    const rawDPContract = (dpContract instanceof DPContract)
      ? dpContract.toJSON()
      : dpContract;

    // TODO: Use validateSchema
    //  https://github.com/epoberezkin/ajv#validateschemaobject-schema---boolean

    const result = validator.validate(
      JsonSchemaValidator.SCHEMAS.META.DP_CONTRACT,
      rawDPContract,
    );

    if (!result.isValid()) {
      return result;
    }

    // Validate indices
    Object.entries(rawDPContract.documents).filter(([, document]) => (
      Object.prototype.hasOwnProperty.call(document, 'indices')
    ))
      .forEach(([documentType, document]) => {
        const indicesFingerprints = [];

        document.indices.forEach((indexDefinition) => {
          const indexPropertyNames = indexDefinition.properties
            .map(property => Object.keys(property)[0]);

          const indicesFingerprint = JSON.stringify(indexDefinition.properties);

          // Ensure index definition uniqueness
          if (indicesFingerprints.includes(indicesFingerprint)) {
            result.addError(
              new DuplicateIndexError(
                rawDPContract,
                documentType,
                indexDefinition,
              ),
            );
          }

          indicesFingerprints.push(indicesFingerprint);

          // Currently, only user-based MN quorums are implemented
          // so we are unable to verify uniqueness among all DPA data, only for user scope.
          // That's why userId prefix for index is temporary required
          if (indexPropertyNames[0] !== '$userId') {
            result.addError(
              new UniqueIndexMustHaveUserIdPrefixError(
                rawDPContract,
                documentType,
                indexDefinition,
              ),
            );

            return;
          }

          // Ensure index properties definition
          const userDefinedProperties = indexPropertyNames.slice(1);

          userDefinedProperties.filter(propertyName => (
            !Object.prototype.hasOwnProperty.call(document.properties, propertyName)
          ))
            .forEach((undefinedPropertyName) => {
              result.addError(
                new UndefinedIndexPropertyError(
                  rawDPContract,
                  documentType,
                  indexDefinition,
                  undefinedPropertyName,
                ),
              );
            });
        });
      });

    return result;
  }

  return validateDPContract;
};
