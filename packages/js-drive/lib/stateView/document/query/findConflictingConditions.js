/**
 * Find duplicate fields in where condition
 *
 * @param {Array[]} conditions
 * @return {Array<string, string[]>[]}
 */
function findConflictingConditions(conditions) {
  // Group operators by fields
  const operatorsByFields = conditions.reduce((fields, [field, operator]) => {
    if (!fields[field]) {
      // eslint-disable-next-line no-param-reassign
      fields[field] = [];
    }

    fields[field].push(operator);

    return fields;
  }, {});

  return Object.entries(operatorsByFields)
    .filter(([, operators]) => {
      // Skip fields with only single one operator
      if (operators.length === 1) {
        return false;
      }

      // Keep fields with two operators
      if (operators.length > 2) {
        return true;
      }

      // Skip fields with range comparison operators
      const [firstOperator, secondOperator] = operators;

      const isFirstLess = firstOperator.startsWith('<');
      const isFirstGreater = firstOperator.startsWith('>');

      if (!isFirstLess && !isFirstGreater) {
        return true;
      }

      const allowedOperatorPrefix = isFirstLess ? '>' : '<';
      const allowedOperators = [allowedOperatorPrefix];

      if (!firstOperator.endsWith('=')) {
        allowedOperators.push(`${allowedOperatorPrefix}=`);
      }

      return !allowedOperators.includes(secondOperator);
    });
}

module.exports = findConflictingConditions;
