const AbstractBasicError = require('../AbstractBasicError');

class DataContractImmutablePropertiesUpdateError extends AbstractBasicError {
  /**
   * @param {{ op: string, path: string }} removedField
   * @param {{ op: string, path: string }} replacedField
   */
  constructor(removedField, replacedField) {
    let message = 'Only $defs, version and documents fields are allowed to be updated.';

    if (removedField) {
      message = `${message} Immutable field '${removedField.path}' has been removed.`;
    }

    if (replacedField) {
      message = `${message} Immutable field '${replacedField.path}' has been changed.`;
    }

    super(message);

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
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
