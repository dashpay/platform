const Document = require('./Document');

const MissingDataContractIdError = require('../errors/MissingDataContractIdError');
const DataContractNotPresentError = require('../errors/DataContractNotPresentError');

const ValidationResult = require('../validation/ValidationResult');

/**
 * @param {StateRepository} stateRepository
 * @return {fetchAndValidateDataContract}
 */
function fetchAndValidateDataContractFactory(stateRepository) {
  /**
   * @typedef fetchAndValidateDataContract
   * @param {Document|RawDocument} document
   * @return {ValidationResult}
   */
  async function fetchAndValidateDataContract(document) {
    const rawDocument = (document instanceof Document) ? document.toJSON() : document;

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawDocument, '$dataContractId')) {
      result.addError(
        new MissingDataContractIdError(rawDocument),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    const dataContractId = rawDocument.$dataContractId;

    const dataContract = await stateRepository.fetchDataContract(dataContractId);

    if (!dataContract) {
      result.addError(
        new DataContractNotPresentError(dataContractId),
      );
    }

    result.setData(dataContract);

    return result;
  }

  return fetchAndValidateDataContract;
}

module.exports = fetchAndValidateDataContractFactory;
