const JsonSchemaValidator = require('../validation/JsonSchemaValidator');

const DataContract = require('./DataContract');

const DuplicateIndexError = require('../errors/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../errors/UndefinedIndexPropertyError');
const InvalidIndexPropertyTypeError = require('../errors/InvalidIndexPropertyTypeError');
const SystemPropertyIndexAlreadyPresentError = require('../errors/SystemPropertyIndexAlreadyPresentError');
const DataContractRestrictedIdentityError = require('../errors/DataContractRestrictedIdentityError');
const UniqueIndicesLimitReachedError = require('../errors/UniqueIndicesLimitReachedError');

const getPropertyDefinitionByPath = require('./getPropertyDefinitionByPath');

const systemProperties = ['$id', '$userId'];
const prebuiltIndices = ['$id'];

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateDataContractMaxDepth} validateDataContractMaxDepth
 * @param {enrichDataContractWithBaseDocument} enrichDataContractWithBaseDocument
 * @param {createDataContract} createDataContract
 * @return {validateDataContract}
 */
module.exports = function validateDataContractFactory(
  jsonSchemaValidator,
  validateDataContractMaxDepth,
  enrichDataContractWithBaseDocument,
  createDataContract,
) {
  /**
   * @typedef validateDataContract
   * @param {DataContract|RawDataContract} dataContract
   * @return {ValidationResult}
   */
  async function validateDataContract(dataContract) {
    const rawDataContract = (dataContract instanceof DataContract)
      ? dataContract.toJSON()
      : dataContract;
    const allowedIdentities = process.env.ALLOWED_IDENTITIES ? process.env.ALLOWED_IDENTITIES.split(',') : [];

    // TODO: Use validateSchema
    //  https://github.com/epoberezkin/ajv#validateschemaobject-schema---boolean

    const result = jsonSchemaValidator.validate(
      JsonSchemaValidator.SCHEMAS.META.DATA_CONTRACT,
      rawDataContract,
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateDataContractMaxDepth(rawDataContract),
    );

    const contractIdentityId = rawDataContract.contractId;

    if (allowedIdentities.length > 0 && !allowedIdentities.includes(contractIdentityId)) {
      result.addError(new DataContractRestrictedIdentityError(rawDataContract));
      return result;
    }

    // Validate Document JSON Schemas
    const enrichedRawDataContract = enrichDataContractWithBaseDocument(
      dataContract,
    );
    const enrichedDataContract = createDataContract(enrichedRawDataContract);

    Object.keys(enrichedRawDataContract.documents).forEach((documentType) => {
      const documentSchemaRef = enrichedDataContract.getDocumentSchemaRef(documentType);

      const additionalSchemas = {
        [enrichedDataContract.getJsonSchemaId()]: enrichedRawDataContract,
      };

      result.merge(
        jsonSchemaValidator.validateSchema(
          documentSchemaRef,
          additionalSchemas,
        ),
      );
    });

    if (!result.isValid()) {
      return result;
    }

    // Validate indices
    Object.entries(rawDataContract.documents).filter(([, documentSchema]) => (
      Object.prototype.hasOwnProperty.call(documentSchema, 'indices')
    ))
      .forEach(([documentType, documentSchema]) => {
        const indicesFingerprints = [];
        let uniqueIndexCount = 0;
        let isUniqueIndexLimitReached = false;

        documentSchema.indices.forEach((indexDefinition) => {
          if (!isUniqueIndexLimitReached && indexDefinition.unique) {
            uniqueIndexCount++;

            if (uniqueIndexCount > UniqueIndicesLimitReachedError.UNIQUE_INDEX_LIMIT) {
              isUniqueIndexLimitReached = true;

              result.addError(new UniqueIndicesLimitReachedError(
                rawDataContract,
                documentType,
              ));
            }
          }

          const indexPropertyNames = indexDefinition.properties
            .map((property) => Object.keys(property)[0]);

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
            const propertyDefinition = (getPropertyDefinitionByPath(
              documentSchema, propertyName,
            ) || {});

            const { type: propertyType } = propertyDefinition;

            let invalidPropertyType;

            if (propertyType === 'object') {
              invalidPropertyType = 'object';
            }

            if (propertyType === 'array') {
              const { items } = propertyDefinition;

              if (Array.isArray(items) || items.type === 'object' || items.type === 'array') {
                invalidPropertyType = 'array';
              }
            }

            if (invalidPropertyType) {
              result.addError(new InvalidIndexPropertyTypeError(
                rawDataContract,
                documentType,
                indexDefinition,
                propertyName,
                invalidPropertyType,
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
            .filter((name) => systemProperties.indexOf(name) === -1);

          userDefinedProperties.filter((propertyName) => (
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
