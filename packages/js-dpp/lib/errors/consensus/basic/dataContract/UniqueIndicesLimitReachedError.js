const AbstractIndexError = require('./AbstractIndexError');

class UniqueIndicesLimitReachedError extends AbstractIndexError {
  /**
   * @param {string} documentType
   */
  constructor(documentType) {
    const message = `'${documentType}' document has more `
      + `than ${UniqueIndicesLimitReachedError.UNIQUE_INDEX_LIMIT} unique indexes`;

    super(
      message,
      documentType,
      {},
    );

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

UniqueIndicesLimitReachedError.UNIQUE_INDEX_LIMIT = 3;

module.exports = UniqueIndicesLimitReachedError;
