const AbstractIndexError = require('./AbstractIndexError');

class UniqueIndicesLimitReached extends AbstractIndexError {
  /**
   * @param {RawDataContract} rawDataContract
   * @param {string} documentType
   */
  constructor(rawDataContract, documentType) {
    const message = `'${documentType}' document has more `
      + `than ${UniqueIndicesLimitReached.UNIQUE_INDEX_LIMIT} unique index definitions`;

    super(
      message,
      rawDataContract,
      documentType,
      {},
    );
  }
}

UniqueIndicesLimitReached.UNIQUE_INDEX_LIMIT = 3;

module.exports = UniqueIndicesLimitReached;
