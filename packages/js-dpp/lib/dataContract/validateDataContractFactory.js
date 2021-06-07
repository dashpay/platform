const JsonSchemaValidator = require('../validation/JsonSchemaValidator');
const ValidationResult = require('../validation/ValidationResult');

const DataContract = require('./DataContract');

const baseDocumentSchema = require('../../schema/document/documentBase');

const DuplicateIndexError = require('../errors/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../errors/UndefinedIndexPropertyError');
const InvalidIndexPropertyTypeError = require('../errors/InvalidIndexPropertyTypeError');
const SystemPropertyIndexAlreadyPresentError = require('../errors/SystemPropertyIndexAlreadyPresentError');
const UniqueIndicesLimitReachedError = require('../errors/UniqueIndicesLimitReachedError');
const InvalidIndexedPropertyConstraintError = require('../errors/InvalidIndexedPropertyConstraintError');
const InvalidCompoundIndexError = require('../errors/InvalidCompoundIndexError');

const getPropertyDefinitionByPathFactory = require('./getPropertyDefinitionByPathFactory');

const convertBuffersToArrays = require('../util/convertBuffersToArrays');

const allowedSystemProperties = ['$id', '$ownerId', '$createdAt', '$updatedAt'];
const prebuiltIndices = ['$id'];

const MAX_INDEXED_STRING_PROPERTY_LENGTH = 1024;

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateDataContractMaxDepth} validateDataContractMaxDepth
 * @param {enrichDataContractWithBaseSchema} enrichDataContractWithBaseSchema
 * @param {validateDataContractPatterns} validateDataContractPatterns
 * @param {RE2} RE2
 * @return {validateDataContract}
 */
module.exports = function validateDataContractFactory(
  jsonSchemaValidator,
  validateDataContractMaxDepth,
  enrichDataContractWithBaseSchema,
  validateDataContractPatterns,
  RE2,
) {
  /**
   * @typedef validateDataContract
   * @param {RawDataContract} rawDataContract
   * @return {ValidationResult}
   */
  async function validateDataContract(rawDataContract) {
    const result = new ValidationResult();

    // Validate Data Contract schema
    result.merge(
      jsonSchemaValidator.validate(
        JsonSchemaValidator.SCHEMAS.META.DATA_CONTRACT,
        convertBuffersToArrays(rawDataContract),
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateDataContractMaxDepth(rawDataContract),
    );

    // Validate regexp patterns are compatible with Re2
    result.merge(
      validateDataContractPatterns(rawDataContract),
    );

    if (!result.isValid()) {
      return result;
    }

    // Validate Document JSON Schemas
    const enrichedDataContract = enrichDataContractWithBaseSchema(
      new DataContract(rawDataContract),
      baseDocumentSchema,
      enrichDataContractWithBaseSchema.PREFIX_BYTE_0,
    );

    Object.keys(enrichedDataContract.getDocuments()).forEach((documentType) => {
      const documentSchemaRef = enrichedDataContract.getDocumentSchemaRef(
        documentType,
      );

      const additionalSchemas = {
        [enrichedDataContract.getJsonSchemaId()]: enrichedDataContract.toJSON(),
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
    Object.entries(enrichedDataContract.documents).filter(([, documentSchema]) => (
      Object.prototype.hasOwnProperty.call(documentSchema, 'indices')
    ))
      .forEach(([documentType, documentSchema]) => {
        const indicesFingerprints = [];
        let uniqueIndexCount = 0;
        let isUniqueIndexLimitReached = false;

        documentSchema.indices.forEach((indexDefinition) => {
          const indexPropertyNames = indexDefinition.properties
            .map((property) => Object.keys(property)[0]);

          // Ensure there are no more than 3 unique indices
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

          // Ensure there are no duplicate system indices
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

          // Ensure index properties are defined in the document
          const userDefinedProperties = indexPropertyNames
            .filter((name) => !allowedSystemProperties.includes(name));
          const getPropertyDefinitionByPath = getPropertyDefinitionByPathFactory(RE2);

          const propertyDefinitionEntities = userDefinedProperties
            .map((propertyName) => (
              [
                propertyName,
                getPropertyDefinitionByPath(documentSchema, propertyName),
              ]
            ));

          const undefinedProperties = propertyDefinitionEntities
            .filter(([, propertyDefinition]) => !propertyDefinition)
            .map(([propertyName]) => {
              result.addError(
                new UndefinedIndexPropertyError(
                  rawDataContract,
                  documentType,
                  indexDefinition,
                  propertyName,
                ),
              );

              return propertyName;
            });

          // Skip further validation if there are undefined properties
          if (undefinedProperties.length > 0) {
            return;
          }

          // Validate indexed property $defs
          propertyDefinitionEntities.forEach(([propertyName, propertyDefinition]) => {
            const {
              type: propertyType,
              byteArray: isByteArray,
            } = propertyDefinition;

            let invalidPropertyType;

            if (propertyType === 'object') {
              invalidPropertyType = 'object';
            }

            if (propertyType === 'array' && !isByteArray) {
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

            if (propertyType === 'string') {
              const { maxLength } = propertyDefinition;

              if (maxLength === undefined) {
                result.addError(
                  new InvalidIndexedPropertyConstraintError(
                    rawDataContract,
                    documentType,
                    indexDefinition,
                    propertyName,
                    'maxLength',
                    'should be set',
                  ),
                );
              }

              if (maxLength !== undefined && maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH) {
                result.addError(
                  new InvalidIndexedPropertyConstraintError(
                    rawDataContract,
                    documentType,
                    indexDefinition,
                    propertyName,
                    'maxLength',
                    `should be less or equal ${MAX_INDEXED_STRING_PROPERTY_LENGTH}`,
                  ),
                );
              }
            }
          });

          // Make sure that compound unique indices contain all fields
          if (indexPropertyNames.length > 1) {
            const requiredFields = documentSchema.required || [];
            const allAreRequired = indexPropertyNames
              .every((propertyName) => requiredFields.includes(propertyName));
            const allAreNotRequired = indexPropertyNames
              .every((propertyName) => !requiredFields.includes(propertyName));

            if (!allAreRequired && !allAreNotRequired) {
              result.addError(
                new InvalidCompoundIndexError(documentType, indexDefinition),
              );
            }
          }

          // Ensure index definition uniqueness
          const indicesFingerprint = JSON.stringify(indexDefinition.properties);

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
        });
      });

    return result;
  }

  return validateDataContract;
};
