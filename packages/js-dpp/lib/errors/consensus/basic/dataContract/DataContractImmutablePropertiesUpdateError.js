const AbstractBasicError = require('../AbstractBasicError');

class DataContractImmutablePropertiesUpdateError extends AbstractBasicError {
  constructor() {
    super('Only $defs, version and documents fields are allowed to be updated');

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = DataContractImmutablePropertiesUpdateError;
