const baseDocumentSchema = require('../../../schema/document/documentBase.json');

const ValidationResult = require('../../validation/ValidationResult');

const convertBuffersToArrays = require('../../util/convertBuffersToArrays');

const InvalidDocumentTypeError = require('../../errors/consensus/basic/document/InvalidDocumentTypeError');
const MissingDocumentTypeError = require('../../errors/consensus/basic/document/MissingDocumentTypeError');

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
        new MissingDocumentTypeError(),
      );

      return result;
    }

    if (!dataContract.isDocumentDefined(rawDocument.$type)) {
      result.addError(
        new InvalidDocumentTypeError(
          rawDocument.$type,
          dataContract.getId().toBuffer(),
        ),
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
        convertBuffersToArrays(rawDocument),
        additionalSchemas,
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    return result;
  }

  return validateDocument;
};
