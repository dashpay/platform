const AbstractBasicError = require('../AbstractBasicError');

class DataContractImmutablePropertiesUpdateError extends AbstractBasicError {
  /**
   * @param {string} operation
   * @param {string} fieldPath
   */
  constructor(operation, fieldPath) {
    let message = 'Only $defs, version and documents fields are allowed to be updated.';

    if (operation === 'remove') {
      message = `${message} Immutable field '${fieldPath}' has been removed.`;
    }

    if (operation === 'replace') {
      message = `${message} Immutable field '${fieldPath}' has been changed.`;
    }

    super(message);

    this.operation = operation;
    this.fieldPath = fieldPath;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get operation
   * @returns {string}
   */
  getOperation() {
    return this.operation;
  }

  /**
   * Get updated field path
   * @returns {string}
   */
  getFieldPath() {
    return this.fieldPath;
  }

  /**
   * Set diff
   * @param {{ op: string, path: string }[]} diff
   */
  setDiff(diff) {
    this.diff = diff;
  }

  /**
   * Get diff
   *
   * @returns {{ op: string, path: string }[]}
   */
  getDiff() {
    return this.diff;
  }
}

module.exports = DataContractImmutablePropertiesUpdateError;
