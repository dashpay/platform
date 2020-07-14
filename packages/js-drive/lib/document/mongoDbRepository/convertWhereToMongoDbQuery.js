const escapeRegExp = require('lodash.escaperegexp');

const convertFieldName = require('./convertFieldName');

const mongoDbOperators = {
  '<': '$lt',
  '<=': '$lte',
  '==': '$eq',
  '>': '$gt',
  '>=': '$gte',
  in: '$in',
  length: '$size',
  startsWith: '$regex',
  elementMatch: '$elemMatch',
  contains: '$all',
};

const dateFieldNames = [
  'createdAt', 'updatedAt',
];

/**
 * @typedef convertWhereToMongoDbQuery
 * @param {Array} where
 * @param {boolean} [isNested=false]
 * @return {Object}
 * @throws InvalidWhereError
 */
function convertWhereToMongoDbQuery(where, isNested = false) {
  const mongoDbQuery = {};

  for (const condition of where) {
    const [field, operator, value] = condition;

    // Convert field name
    const mongoDbField = isNested ? field : convertFieldName(field);

    // Convert operator
    const mongoDbOperator = mongoDbOperators[operator];

    // Convert value
    let mongoDbValue;
    switch (operator) {
      case 'startsWith': {
        mongoDbValue = new RegExp(`^${escapeRegExp(value)}`);

        break;
      }
      case 'elementMatch': {
        mongoDbValue = convertWhereToMongoDbQuery(value, true);

        break;
      }
      case 'contains': {
        mongoDbValue = Array.isArray(value) ? value : [value];

        break;
      }
      default: {
        mongoDbValue = value;
      }
    }

    if (!mongoDbQuery[mongoDbField]) {
      mongoDbQuery[mongoDbField] = {};
    }

    if (dateFieldNames.includes(mongoDbField)) {
      mongoDbValue = new Date(mongoDbValue);
    }

    mongoDbQuery[mongoDbField][mongoDbOperator] = mongoDbValue;
  }

  return mongoDbQuery;
}

module.exports = convertWhereToMongoDbQuery;
