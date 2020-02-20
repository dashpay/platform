const AbstractIndexError = require('./AbstractIndexError');

class UniqueIndicesLimitReachedError extends AbstractIndexError {
  /**
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   */
  constructor(rawDataContract, documentType) {
    const message = `'${documentType}' document has more `
      + `than ${UniqueIndicesLimitReachedError.UNIQUE_INDEX_LIMIT} unique index definitions`;

    super(
      message,
      rawDataContract,
      documentType,
      {},
    );
  }
}

UniqueIndicesLimitReachedError.UNIQUE_INDEX_LIMIT = 3;

module.exports = UniqueIndicesLimitReachedError;
