const Document = require('./Document');
const baseDocumentSchema = require('../../schema/document/documentBase');

const ValidationResult = require('../validation/ValidationResult');

const InvalidDocumentTypeError = require('../errors/InvalidDocumentTypeError');
const MissingDocumentTypeError = require('../errors/MissingDocumentTypeError');
const MismatchDocumentContractIdAndDataContractError = require('../errors/MismatchDocumentContractIdAndDataContractError');

/**
 * @param {JsonSchemaValidator} validator
 * @param {enrichDataContractWithBaseSchema} enrichDataContractWithBaseSchema
 * @return {validateDocument}
 */
module.exports = function validateDocumentFactory(
  validator,
  enrichDataContractWithBaseSchema,
) {
  /**
   * @typedef validateDocument
   * @param {Document|RawDocument} document
   * @param {DataContract} dataContract
   * @return {ValidationResult}
   */
  function validateDocument(document, dataContract) {
    /**
     * @type {RawDocument}
     */
    const rawDocument = (document instanceof Document) ? document.toJSON() : document;

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

    const documentSchemaRef = dataContract.getDocumentSchemaRef(rawDocument.$type);

    const enrichedDataContract = enrichDataContractWithBaseSchema(
      dataContract,
      baseDocumentSchema,
    );

    const additionalSchemas = {
      [dataContract.getJsonSchemaId()]: enrichedDataContract,
    };

    result.merge(
      validator.validate(
        documentSchemaRef,
        rawDocument,
        additionalSchemas,
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    if (rawDocument.$dataContractId !== dataContract.getId()) {
      result.addError(
        new MismatchDocumentContractIdAndDataContractError(rawDocument, dataContract),
      );
    }

    return result;
  }

  return validateDocument;
};
