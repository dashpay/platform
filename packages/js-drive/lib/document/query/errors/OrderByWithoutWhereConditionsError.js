const ValidationError = require('./ValidationError');

class OrderByWithoutWhereConditionsError extends ValidationError {
  constructor() {
    super('Use of "orderBy" without "where" conditions is not allowed');
  }
}

module.exports = OrderByWithoutWhereConditionsError;
