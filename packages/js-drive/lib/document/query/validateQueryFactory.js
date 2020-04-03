const Ajv = require('ajv');

const ValidationResult = require('./ValidationResult');

const JsonSchemaValidationError = require('./errors/JsonSchemaValidationError');
const ConflictingConditionsError = require('./errors/ConflictingConditionsError');
const DuplicateSortingFieldError = require('./errors/DuplicateSortingFieldError');
const NestedSystemFieldError = require('./errors/NestedSystemFieldError');
const NestedElementMatchError = require('./errors/NestedElementMatchError');
const NotIndexedFieldError = require('./errors/NotIndexedFieldError');
const NotIndexedOrderByError = require('./errors/NotIndexedOrderByError');

const jsonSchema = require('./jsonSchema');

/**
 * @param {findConflictingConditions} findConflictingConditions
 * @param {getIndexedFieldsFromDocumentSchema} getIndexedFieldsFromDocumentSchema
 * @param {findNotIndexedFields} findNotIndexedFields
 * @param {findNotIndexedOrderByFields} findNotIndexedOrderByFields
 *
 * @return {validateQuery}
 */
function validateQueryFactory(
  findConflictingConditions,
  getIndexedFieldsFromDocumentSchema,
  findNotIndexedFields,
  findNotIndexedOrderByFields,
) {
  const ajv = new Ajv();

  const validateWithJsonSchema = ajv.compile(jsonSchema);

  /**
   * Validate fetchDocuments query
   *
   * @typedef validateQuery
   * @param {Object} query
   * @param {Object} documentSchema
   * @return {ValidationResult}
   */
  function validateQuery(query, documentSchema) {
    const result = new ValidationResult();

    const isValid = validateWithJsonSchema(query);

    if (!isValid) {
      return result.addError(
        ...validateWithJsonSchema.errors.map((e) => new JsonSchemaValidationError(e)),
      );
    }

    const dataContractIndexFields = getIndexedFieldsFromDocumentSchema(documentSchema);

    // Additional validations for where conditions
    if (query.where) {
      // Find conflicting conditions
      result.addError(
        ...findConflictingConditions(query.where)
          .map(([field, operators]) => new ConflictingConditionsError(field, operators)),
      );

      // Check all fields having index
      result.addError(
        ...findNotIndexedFields(dataContractIndexFields, query.where)
          .map((field) => new NotIndexedFieldError(field)),
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

      result.addError(
        ...findNotIndexedOrderByFields(dataContractIndexFields, query.orderBy, query.where)
          .map((field) => new NotIndexedOrderByError(field)),
      );
    }

    return result;
  }

  return validateQuery;
}

module.exports = validateQueryFactory;
