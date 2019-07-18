const Ajv = require('ajv');

const ValidationResult = require('./ValidationResult');

const JsonSchemaValidationError = require('./errors/JsonSchemaValidationError');
const ConflictingConditionsError = require('./errors/ConflictingConditionsError');
const DuplicateSortingFieldError = require('./errors/DuplicateSortingFieldError');
const NestedSystemFieldError = require('./errors/NestedSystemFieldError');
const NestedElementMatchError = require('./errors/NestedElementMatchError');

const jsonSchema = require('./jsonSchema');

/**
 * @return {validateQuery}
 */
function validateQueryFactory(findConflictingConditions) {
  const ajv = new Ajv();

  const validateWithJsonSchema = ajv.compile(jsonSchema);

  /**
   * Validate fetchDocuments query
   *
   * @typedef validateQuery
   * @param {Object} query
   * @return {ValidationResult}
   */
  function validateQuery(query) {
    const result = new ValidationResult();

    const isValid = validateWithJsonSchema(query);

    if (!isValid) {
      return result.addError(
        ...validateWithJsonSchema.errors.map(e => new JsonSchemaValidationError(e)),
      );
    }

    // Additional validations for where conditions
    if (query.where) {
      // Find conflicting conditions
      result.addError(
        ...findConflictingConditions(query.where)
          .map(([field, operators]) => new ConflictingConditionsError(field, operators)),
      );

      // Check nested elementMatch
      const elementMatch = query.where.find(([, operator]) => operator === 'elementMatch');

      if (elementMatch) {
        // Find conflicting conditions in nested elementMatch
        result.addError(
          ...findConflictingConditions(elementMatch)
            .map(([field, operators]) => new ConflictingConditionsError(field, operators)),
        );

        const [, , elementMatchValue] = elementMatch;

        // Find system fields
        result.addError(
          ...elementMatchValue.filter(([field]) => field.startsWith('$'))
            .map(([field]) => new NestedSystemFieldError(field)),
        );

        // Find nested elementMatch
        const nestedElementMatch = elementMatchValue.find(([, operator]) => operator === 'elementMatch');

        // Report error if found
        if (nestedElementMatch) {
          const [field] = nestedElementMatch;

          result.addError(
            new NestedElementMatchError(field),
          );
        }
      }
    }

    // Additional validations for orderBy
    if (query.orderBy) {
      // Find duplicates in orderBy
      const orderByFields = new Set();

      result.addError(
        ...query.orderBy
          .filter(([field]) => {
            const isDuplicatedField = orderByFields.has(field);
            if (!isDuplicatedField) {
              orderByFields.add(field);
            }
            return isDuplicatedField;
          })
          .map(([field]) => new DuplicateSortingFieldError(field)),
      );
    }

    return result;
  }

  return validateQuery;
}

module.exports = validateQueryFactory;
