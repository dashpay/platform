const JsonSchemaValidator = require('../../validation/JsonSchemaValidator');
const ValidationResult = require('../../validation/ValidationResult');

const DataContract = require('../DataContract');

const baseDocumentSchema = require('../../../schema/document/documentBase.json');

const DuplicateIndexError = require('../../errors/consensus/basic/dataContract/DuplicateIndexError');
const UndefinedIndexPropertyError = require('../../errors/consensus/basic/dataContract/UndefinedIndexPropertyError');
const InvalidIndexPropertyTypeError = require('../../errors/consensus/basic/dataContract/InvalidIndexPropertyTypeError');
const SystemPropertyIndexAlreadyPresentError = require('../../errors/consensus/basic/dataContract/SystemPropertyIndexAlreadyPresentError');
const UniqueIndicesLimitReachedError = require('../../errors/consensus/basic/dataContract/UniqueIndicesLimitReachedError');
const InvalidIndexedPropertyConstraintError = require('../../errors/consensus/basic/dataContract/InvalidIndexedPropertyConstraintError');
const InvalidCompoundIndexError = require('../../errors/consensus/basic/dataContract/InvalidCompoundIndexError');

const convertBuffersToArrays = require('../../util/convertBuffersToArrays');
const DuplicateIndexNameError = require('../../errors/consensus/basic/dataContract/DuplicateIndexNameError');

const allowedIndexSystemProperties = ['$ownerId', '$createdAt', '$updatedAt'];
const notAllowedIndexProperties = ['$id'];

const MAX_INDEXED_STRING_PROPERTY_LENGTH = 64;
const MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH = 256;
const MAX_INDEXED_ARRAY_ITEMS = 1024;

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateDataContractMaxDepth} validateDataContractMaxDepth
 * @param {enrichDataContractWithBaseSchema} enrichDataContractWithBaseSchema
 * @param {validateDataContractPatterns} validateDataContractPatterns
 * @param {validateProtocolVersion} validateProtocolVersion
 * @param {getPropertyDefinitionByPath} getPropertyDefinitionByPath
 * @return {validateDataContract}
 */
module.exports = function validateDataContractFactory(
  jsonSchemaValidator,
  validateDataContractMaxDepth,
  enrichDataContractWithBaseSchema,
  validateDataContractPatterns,
  validateProtocolVersion,
  getPropertyDefinitionByPath,
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
      validateProtocolVersion(rawDataContract.protocolVersion),
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

        // Ensure index names are unique
        const indexNames = documentSchema.indices.map((indexDefinition) => indexDefinition.name);
        const [nonUniqueIndexName] = indexNames.filter(
          (indexName, i) => indexNames.indexOf(indexName) !== i,
        );

        if (nonUniqueIndexName !== undefined) {
          result.addError(new DuplicateIndexNameError(
            documentType,
            nonUniqueIndexName,
          ));
        }

        documentSchema.indices.forEach((indexDefinition) => {
          const indexPropertyNames = indexDefinition.properties
            .map((property) => Object.keys(property)[0]);

          // Ensure there are no more than 3 unique indices
          if (!isUniqueIndexLimitReached && indexDefinition.unique) {
            uniqueIndexCount++;

            if (uniqueIndexCount > UniqueIndicesLimitReachedError.UNIQUE_INDEX_LIMIT) {
              isUniqueIndexLimitReached = true;

              result.addError(new UniqueIndicesLimitReachedError(
                documentType,
              ));
            }
          }

          // Ensure there are no duplicate system indices
          notAllowedIndexProperties
            .forEach((propertyName) => {
              if (indexPropertyNames.includes(propertyName)) {
                result.addError(new SystemPropertyIndexAlreadyPresentError(
                  documentType,
                  indexDefinition,
                  propertyName,
                ));
              }
            });

          // Ensure index properties are defined in the document
          const userDefinedProperties = indexPropertyNames
            .filter((name) => !allowedIndexSystemProperties.includes(name));

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

            // const { items, prefixItems } = propertyDefinition;

            // Validate arrays contain scalar values or have the same types
            if (propertyType === 'array' && !isByteArray) {
              invalidPropertyType = 'array';

            // const isInvalidPrefixItems = prefixItems
            //   && (
            // prefixItems.some((prefixItem) =>
              // prefixItem.type === 'object' || prefixItem.type === 'array')
            //     || !prefixItems.every((prefixItem) => prefixItem.type === prefixItems[0].type)
            //   );
            //
            // const isInvalidItemTypes = items.type === 'object' || items.type === 'array';
            //
            // if (isInvalidPrefixItems || isInvalidItemTypes) {
            //   invalidPropertyType = 'array';
            // }
            }

            if (invalidPropertyType) {
              result.addError(new InvalidIndexPropertyTypeError(
                documentType,
                indexDefinition,
                propertyName,
                invalidPropertyType,
              ));
            }

            // Validate sting length inside arrays
            // if (!invalidPropertyType && propertyType === 'array' && !isByteArray) {
            //   const isInvalidPrefixItems = prefixItems && prefixItems.some((prefixItem) => (
            //     prefixItem.type === 'string'
            //     && (
            // !prefixItem.maxLength || prefixItem.maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH
            //     )
            //   ));
            //
            //   const isInvalidItemTypes = items.type === 'string' && (
            //     !items.maxLength || items.maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH
            //   );
            //
            //   if (isInvalidPrefixItems || isInvalidItemTypes) {
            //     result.addError(
            //       new InvalidIndexedPropertyConstraintError(
            //         documentType,
            //         indexDefinition,
            //         propertyName,
            //         'maxLength',
            //         `should be less or equal ${MAX_INDEXED_STRING_PROPERTY_LENGTH}`,
            //       ),
            //     );
            //   }
            // }
            //
            if (!invalidPropertyType && propertyType === 'array') {
              const { maxItems } = propertyDefinition;

              let maxLimit;
              if (isByteArray) {
                maxLimit = MAX_INDEXED_BYTE_ARRAY_PROPERTY_LENGTH;
              } else {
                maxLimit = MAX_INDEXED_ARRAY_ITEMS;
              }

              if ((maxItems === undefined || maxItems > maxLimit)) {
                result.addError(
                  new InvalidIndexedPropertyConstraintError(
                    documentType,
                    indexDefinition,
                    propertyName,
                    'maxItems',
                    `should be less or equal ${maxLimit}`,
                  ),
                );
              }
            }

            if (propertyType === 'string') {
              const { maxLength } = propertyDefinition;

              if (maxLength === undefined || maxLength > MAX_INDEXED_STRING_PROPERTY_LENGTH) {
                result.addError(
                  new InvalidIndexedPropertyConstraintError(
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
