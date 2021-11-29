const AbstractBasicError = require('../AbstractBasicError');

class DataContractImmutablePropertiesUpdateError extends AbstractBasicError {
  /**
   * @param {{ op: string, path: string }[]} diff
   */
  constructor(diff) {
    let message = 'Only $defs, version and documents fields are allowed to be updated.';

    const removedField = diff.filter(({ op }) => op === 'remove')[0];
    const replacedField = diff.filter(({ op }) => op === 'replace')[0];

    if (removedField) {
      message = `${message} Immutable field '${removedField.path}' has been removed.`;
    }

    if (replacedField) {
      message = `${message} Immutable field '${replacedField.path}' has been changed.`;
    }

    super(message);

    this.diff = diff;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
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
