const { default: Ajv } = require('ajv/dist/2020');
const defineAjvKeywords = require('ajv-keywords');

const ValidationResult = require('./ValidationResult');

const JsonSchemaValidationError = require('./errors/JsonSchemaValidationError');
const ConflictingConditionsError = require('./errors/ConflictingConditionsError');

const jsonSchema = require('./jsonSchema');

const NotIndexedPropertiesInWhereConditionsError = require('./errors/NotIndexedPropertiesInWhereConditionsError');
const InvalidPropertiesInOrderByError = require('./errors/InvalidPropertiesInOrderByError');

/**
 * @param {findConflictingConditions} findConflictingConditions
 * @param {findAppropriateIndex} findAppropriateIndex
 * @param {sortWhereClausesAccordingToIndex} sortWhereClausesAccordingToIndex
 * @return {validateQuery}
 */
function validateQueryFactory(
  findConflictingConditions,
  findAppropriateIndex,
  sortWhereClausesAccordingToIndex,
) {
  const ajv = defineAjvKeywords(new Ajv({
    strictTypes: true,
    strictTuples: true,
    strictRequired: true,
    addUsedSchema: false,
    strict: true,
  }), ['instanceof']);

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

    let sortedWhereClauses = [];
    let appropriateIndex;

    // Where conditions must follow document indices
    if (query.where) {
      // Find conflicting conditions
      result.addError(
        ...findConflictingConditions(query.where)
          .map(([field, operators]) => new ConflictingConditionsError(field, operators)),
      );

      appropriateIndex = findAppropriateIndex(query, documentSchema);

      if (!appropriateIndex) {
        result.addError(new NotIndexedPropertiesInWhereConditionsError());
      }

      sortedWhereClauses = sortWhereClausesAccordingToIndex(query.where, appropriateIndex);
    }

    // Sorting is allowed only for the last indexed property
    if (query.orderBy) {
      if (!query.where) {
        result.addError(new InvalidPropertiesInOrderByError());

        return result;
      }

      if (query.orderBy.length > 1) {
        result.addError(new InvalidPropertiesInOrderByError());

        return result;
      }

      const lastCondition = sortedWhereClauses[sortedWhereClauses.length - 1];

      const [property, operator] = lastCondition;

      if (!operator.includes('<') && !operator.includes('>')) {
        result.addError(new InvalidPropertiesInOrderByError());

        return result;
      }

      const orderedProperty = query.orderBy[0][0];

      if (property !== orderedProperty) {
        result.addError(new InvalidPropertiesInOrderByError());

        return result;
      }
    }

    return result;
  }

  return validateQuery;
}

module.exports = validateQueryFactory;
