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
 * @param {enrichContractWithBaseDocument} enrichContractWithBaseDocument
 * @return {validateDocument}
 */
module.exports = function validateDocumentFactory(
  validator,
  enrichContractWithBaseDocument,
) {
  /**
   * @typedef validateDocument
   * @param {Object|Document} document
   * @param {Contract} contract
   * @return {ValidationResult}
   */
  function validateDocument(document, contract) {
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

    if (!contract.isDocumentDefined(rawDocument.$type)) {
      result.addError(
        new InvalidDocumentTypeError(rawDocument.$type, contract),
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
      const documentSchemaRef = contract.getDocumentSchemaRef(rawDocument.$type);

      const enrichedContract = enrichContractWithBaseDocument(contract);

      const additionalSchemas = {
        [contract.getJsonSchemaId()]: enrichedContract,
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
