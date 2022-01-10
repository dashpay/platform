const ValidationError = require('./ValidationError');

class NotIndexedPropertiesInWhereConditionsError extends ValidationError {
  constructor() {
    super('Properties in where conditions must be defined as a document index');
  }
}

module.exports = NotIndexedPropertiesInWhereConditionsError;
