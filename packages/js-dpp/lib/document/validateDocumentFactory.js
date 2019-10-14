const Document = require('./Document');

const documentBaseSchema = require('../../schema/base/document');

const ValidationResult = require('../validation/ValidationResult');

const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');
const MissingDocumentTypeError = require('../errors/MissingDocumentTypeError');
const InvalidDocumentEntropyError = require('../errors/InvalidDocumentEntropyError');

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
   * @param {boolean} [options.action]
   * @return {ValidationResult}
   */
  function validateDocument(document, dataContract, options = { }) {
    /**
     * @type {RawDocument}
     */
    let rawDocument;
    let { action } = options;

    if (document instanceof Document) {
      rawDocument = document.toJSON();

      if (action === undefined) {
        action = document.getAction();
      }
    } else {
      rawDocument = document;
    }

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawDocument, '$type')) {
      result.addError(
        new MissingDocumentTypeError(rawDocument),
      );

      return result;
    }

    if (!dataContract.isDocumentDefined(rawDocument.$type)) {
      result.addError(
        new InvalidDocumentTypeError(rawDocument.$type, dataContract),
      );

      return result;
    }

    if (action === Document.ACTIONS.DELETE) {
      const schemaValidationResult = validator.validate(
        documentBaseSchema,
        rawDocument,
      );

      result.merge(schemaValidationResult);
    } else {
      const documentSchemaRef = dataContract.getDocumentSchemaRef(rawDocument.$type);

      const enrichedDataContract = enrichDataContractWithBaseDocument(
        dataContract,
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

    if (!entropy.validate(rawDocument.$entropy)) {
      result.addError(
        new InvalidDocumentEntropyError(rawDocument),
      );
    }

    return result;
  }

  return validateDocument;
};
