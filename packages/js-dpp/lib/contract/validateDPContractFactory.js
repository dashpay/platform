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
    Object.entries(rawDPContract.dpObjectsDefinition).filter(([, dpObjectDefinition]) => (
      Object.prototype.hasOwnProperty.call(dpObjectDefinition, 'indices')
    ))
      .forEach(([dpObjectType, dpObjectDefinition]) => {
        const indicesFingerprints = [];

        dpObjectDefinition.indices.forEach((indexDefinition) => {
          const indexPropertyNames = Object.keys(indexDefinition.properties);

          const indicesFingerprint = JSON.stringify(indexDefinition.properties);

          // Ensure index definition uniqueness
          if (indicesFingerprints.includes(indicesFingerprint)) {
            result.addError(
              new DuplicateIndexError(
                rawDPContract,
                dpObjectType,
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
                dpObjectType,
                indexDefinition,
              ),
            );

            return;
          }

          // Ensure index properties definition
          const userDefinedProperties = indexPropertyNames.slice(1);

          userDefinedProperties.filter(propertyName => (
            !Object.prototype.hasOwnProperty.call(dpObjectDefinition.properties, propertyName)
          ))
            .forEach((undefinedPropertyName) => {
              result.addError(
                new UndefinedIndexPropertyError(
                  rawDPContract,
                  dpObjectType,
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
