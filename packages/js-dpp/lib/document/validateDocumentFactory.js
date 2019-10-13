const Document = require('./Document');

const documentBaseSchema = require('../../schema/base/document');

const ValidationResult = require('../validation/ValidationResult');

const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');
const MissingDocumentTypeError = require('../errors/MissingDocumentTypeError');
const MissingDocumentActionError = require('../errors/MissingDocumentActionError');
const InvalidDocumentScopeIdError = require('../errors/InvalidDocumentScopeIdError');

const entropy = require('../util/entropy');

/**
 * @param {JsonSchemaValidator} validator
 * @param {enrichDataContractWithBaseDocument} enrichDataContractWithBaseDocument
 * @return {validateDocument}
 */
module.exports = function validateDocumentFactory(
  validator,
  enrichDataContractWithBaseDocument,
) {
  /**
   * @typedef validateDocument
   * @param {Document|RawDocument} document
   * @param {DataContract} dataContract
   * @param {Object} [options]
   * @param {boolean} [options.allowMeta=true]
   * @return {ValidationResult}
   */
  function validateDocument(document, dataContract, options = { allowMeta: true }) {
    const rawDocument = (document instanceof Document) ? document.toJSON() : document;

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawDocument, '$type')) {
      result.addError(
        new MissingDocumentTypeError(rawDocument),
      );

      return result;
    }

    if (!Object.prototype.hasOwnProperty.call(rawDocument, '$action')) {
      result.addError(
        new MissingDocumentActionError(rawDocument),
      );

      return result;
    }

    if (!dataContract.isDocumentDefined(rawDocument.$type)) {
      result.addError(
        new InvalidDocumentTypeError(rawDocument.$type, dataContract),
      );

      return result;
    }

    if (rawDocument.$action === Document.ACTIONS.DELETE) {
      const schemaValidationResult = validator.validate(
        documentBaseSchema,
        rawDocument,
      );

      result.merge(schemaValidationResult);
    } else {
      const documentSchemaRef = dataContract.getDocumentSchemaRef(rawDocument.$type);

      const enrichedDataContract = enrichDataContractWithBaseDocument(
        dataContract,
        options.allowMeta ? [] : ['$meta'],
      );

      const additionalSchemas = {
        [dataContract.getJsonSchemaId()]: enrichedDataContract,
      };

      const schemaValidationResult = validator.validate(
        documentSchemaRef,
        rawDocument,
        additionalSchemas,
      );

      result.merge(schemaValidationResult);
    }

    if (!entropy.validate(rawDocument.$scopeId)) {
      result.addError(
        new InvalidDocumentScopeIdError(rawDocument),
      );
    }

    return result;
  }

  return validateDocument;
};
