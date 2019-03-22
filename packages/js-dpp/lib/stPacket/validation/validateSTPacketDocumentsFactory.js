const ValidationResult = require('../../validation/ValidationResult');

const DuplicateDocumentsError = require('../../errors/DuplicateDocumentsError');
const InvalidContractError = require('../../errors/InvalidContractError');

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
   * @param {Contract} contract
   * @return {ValidationResult}
   */
  function validateSTPacketDocuments(rawSTPacket, contract) {
    const { documents: rawDocuments } = rawSTPacket;

    const result = new ValidationResult();

    if (contract.getId() !== rawSTPacket.contractId) {
      result.addError(
        new InvalidContractError(contract, rawSTPacket),
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
      contract,
    );
    if (duplicateDocumentsByIndices.length > 0) {
      result.addError(
        new DuplicateDocumentsError(duplicateDocumentsByIndices),
      );
    }

    rawDocuments.forEach((rawDocument) => {
      result.merge(
        validateDocument(rawDocument, contract),
      );
    });

    return result;
  }

  return validateSTPacketDocuments;
}

module.exports = validateSTPacketDocumentsFactory;
