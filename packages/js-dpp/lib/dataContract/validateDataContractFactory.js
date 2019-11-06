const JsonSchemaValidator = require('../validation/JsonSchemaValidator');

const DataContract = require('./DataContract');

const DuplicateIndexError = require('../errors/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../errors/UndefinedIndexPropertyError');
const InvalidIndexPropertyTypeError = require('../errors/InvalidIndexPropertyTypeError');
const SystemPropertyIndexAlreadyPresentError = require('../errors/SystemPropertyIndexAlreadyPresentError');

const getPropertyDefinitionByPath = require('./getPropertyDefinitionByPath');

const systemProperties = ['$id', '$userId'];
const prebuiltIndices = ['$id'];

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

          prebuiltIndices
            .forEach((propertyName) => {
              const isSingleIndex = indexPropertyNames.length === 1
                    && indexPropertyNames[0] === propertyName;

              if (isSingleIndex) {
                result.addError(new SystemPropertyIndexAlreadyPresentError(
                  rawDataContract,
                  documentType,
                  indexDefinition,
                  propertyName,
                ));
              }
            });

          indexPropertyNames.forEach((propertyName) => {
            const { type: propertyType } = (getPropertyDefinitionByPath(
              documentSchema, propertyName,
            ) || {});

            if (propertyType === 'object') {
              result.addError(new InvalidIndexPropertyTypeError(
                rawDataContract,
                documentType,
                indexDefinition,
                propertyName,
                'object',
              ));
            }
          });

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
          const userDefinedProperties = indexPropertyNames
            .filter(name => systemProperties.indexOf(name) === -1);

          userDefinedProperties.filter(propertyName => (
            !getPropertyDefinitionByPath(documentSchema, propertyName)
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
