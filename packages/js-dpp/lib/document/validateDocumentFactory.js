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
   * @param {RawDocument} rawDocument
   * @param {DataContract} dataContract
   * @return {ValidationResult}
   */
  function validateDocument(rawDocument, dataContract) {
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

    const enrichedDataContract = enrichDataContractWithBaseSchema(
      dataContract,
      baseDocumentSchema,
      enrichDataContractWithBaseSchema.PREFIX_BYTE_0,
    );

    const documentSchemaRef = enrichedDataContract.getDocumentSchemaRef(
      rawDocument.$type,
    );

    const additionalSchemas = {
      [enrichedDataContract.getJsonSchemaId()]: enrichedDataContract.toJSON(),
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

    if (!rawDocument.$dataContractId.equals(dataContract.getId())) {
      result.addError(
        new MismatchDocumentContractIdAndDataContractError(rawDocument, dataContract),
      );
    }

    return result;
  }

  return validateDocument;
};
