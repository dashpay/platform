const ValidationResult = require('../../validation/ValidationResult');

const DuplicateDocumentsError = require('../../errors/DuplicateDocumentsError');
const InvalidDataContractError = require('../../errors/InvalidDataContractError');

/**
 * @param {validateDocument} validateDocument
 * @param {findDuplicateDocuments} findDuplicateDocuments
 * @param {findDuplicateDocumentsByIndices} findDuplicateDocumentsByIndices
 * @return {validateSTPacketDocuments}
 */
function validateSTPacketDocumentsFactory(
  validateDocument,
  findDuplicateDocuments,
  findDuplicateDocumentsByIndices,
) {
  /**
   * @typedef validateSTPacketDocuments
   * @param {RawSTPacket} rawSTPacket
   * @param {DataContract} dataContract
   * @return {ValidationResult}
   */
  function validateSTPacketDocuments(rawSTPacket, dataContract) {
    const { documents: rawDocuments } = rawSTPacket;

    const result = new ValidationResult();

    if (dataContract.getId() !== rawSTPacket.contractId) {
      result.addError(
        new InvalidDataContractError(dataContract, rawSTPacket),
      );
    }

    const duplicateDocuments = findDuplicateDocuments(rawDocuments);
    if (duplicateDocuments.length) {
      result.addError(
        new DuplicateDocumentsError(duplicateDocuments),
      );
    }

    const duplicateDocumentsByIndices = findDuplicateDocumentsByIndices(
      rawDocuments,
      dataContract,
    );
    if (duplicateDocumentsByIndices.length > 0) {
      result.addError(
        new DuplicateDocumentsError(duplicateDocumentsByIndices),
      );
    }

    rawDocuments.forEach((rawDocument) => {
      result.merge(
        validateDocument(rawDocument, dataContract),
      );
    });

    return result;
  }

  return validateSTPacketDocuments;
}

module.exports = validateSTPacketDocumentsFactory;
