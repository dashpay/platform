const Document = require('./Document');

const MissingDocumentContractIdError = require('../errors/MissingDocumentContractIdError');
const DataContractNotPresentError = require('../errors/DataContractNotPresentError');

const ValidationResult = require('../validation/ValidationResult');

/**
 * @param {DataProvider} dataProvider
 * @return {fetchAndValidateDataContract}
 */
function fetchAndValidateDataContractFactory(dataProvider) {
  /**
   * @typedef fetchAndValidateDataContract
   * @param {Document|RawDocument} document
   * @return {ValidationResult}
   */
  async function fetchAndValidateDataContract(document) {
    const rawDocument = (document instanceof Document) ? document.toJSON() : document;

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawDocument, '$contractId')) {
      result.addError(
        new MissingDocumentContractIdError(rawDocument),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    const dataContractId = rawDocument.$contractId;

    const dataContract = await dataProvider.fetchDataContract(dataContractId);

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
