const JsonSchemaValidator = require('../validation/JsonSchemaValidator');

const DataContract = require('./DataContract');

const DuplicateIndexError = require('../errors/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../errors/UndefinedIndexPropertyError');

/**
 * @param validator
 * @return {validateDataContract}
 */
module.exports = function validateDataContractFactory(validator) {
  /**
   * @typedef validateDataContract
   * @param {DataContract|RawDataContract} dataContract
   * @return {ValidationResult}
   */
  function validateDataContract(dataContract) {
    const rawDataContract = (dataContract instanceof DataContract)
      ? dataContract.toJSON()
      : dataContract;

    // TODO: Use validateSchema
    //  https://github.com/epoberezkin/ajv#validateschemaobject-schema---boolean

    const result = validator.validate(
      JsonSchemaValidator.SCHEMAS.META.DATA_CONTRACT,
      rawDataContract,
    );

    if (!result.isValid()) {
      return result;
    }

    // Validate indices
    Object.entries(rawDataContract.documents).filter(([, documentSchema]) => (
      Object.prototype.hasOwnProperty.call(documentSchema, 'indices')
    ))
      .forEach(([documentType, documentSchema]) => {
        const indicesFingerprints = [];

        documentSchema.indices.forEach((indexDefinition) => {
          const indexPropertyNames = indexDefinition.properties
            .map(property => Object.keys(property)[0]);

          const indicesFingerprint = JSON.stringify(indexDefinition.properties);

          // Ensure index definition uniqueness
          if (indicesFingerprints.includes(indicesFingerprint)) {
            result.addError(
              new DuplicateIndexError(
                rawDataContract,
                documentType,
                indexDefinition,
              ),
            );
          }

          indicesFingerprints.push(indicesFingerprint);

          // Ensure index properties definition
          const userDefinedProperties = indexPropertyNames.slice(1);

          userDefinedProperties.filter(propertyName => (
            !Object.prototype.hasOwnProperty.call(documentSchema.properties, propertyName)
          ))
            .forEach((undefinedPropertyName) => {
              result.addError(
                new UndefinedIndexPropertyError(
                  rawDataContract,
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

  return validateDataContract;
};
