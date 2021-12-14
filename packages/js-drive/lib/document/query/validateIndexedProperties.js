const ValidationResult = require('./ValidationResult');
const NotIndexedFieldError = require('./errors/NotIndexedFieldError');
const FieldsFromMultipleIndicesError = require('./errors/FieldsFromMultipleIndicesError');

/**
 * Validate query to search by only indexed fields
 *
 * @typedef validateIndexedProperties
 * @param {{field, direction}[][]} dataContractIndexFields
 * @param {indexStructure[]} sortingFields
 * @param {[string, string, string][]} whereConditions
 * @returns {ValidationResult}
 */
function validateIndexedProperties(
  dataContractIndexFields,
  sortingFields = [],
  whereConditions = [],
) {
  const result = new ValidationResult();

  // convert conditions to better format
  const queryFields = whereConditions
    .reduce((fields, [field, operator, elementMatchValue]) => {
      let fieldsToAdd;

      if (operator === 'elementMatch') {
        fieldsToAdd = elementMatchValue.map(([item]) => `${field}.${item}`);
      } else {
        fieldsToAdd = [field];
      }

      return fields.concat(fieldsToAdd);
    }, []);

  let indexToUse;

  // validate fields
  const notIndexedFields = queryFields
    .filter((field) => {
      const fieldHasIndex = dataContractIndexFields
      // find our field in indices
        .find((index) => index
          // search through compound index
          .find((element, i) => {
            const [indexField] = Object.keys(element);
            if (indexField !== field) {
              return false;
            }

            if (indexToUse === undefined) {
              indexToUse = index;
            } else if (indexToUse !== index) {
              // another index has already been used
              result.addError(new FieldsFromMultipleIndicesError(field));
            }

            // get previous fields from compound index
            const compoundFields = index.slice(0, i);

            // check that we have each previous compound index field in our condition
            return compoundFields.every((item) => {
              const [compoundField] = Object.keys(item);

              return queryFields.includes(compoundField);
            });
          }));

      return !fieldHasIndex;
    });

  notIndexedFields.forEach((field) => result.addError(new NotIndexedFieldError(field)));

  sortingFields
    .forEach(([sortingField]) => {
      dataContractIndexFields
        // find our field in indices
        .forEach((index) => index
          .forEach((element) => {
            const [indexField] = Object.keys(element);
            if (indexField !== sortingField) {
              return;
            }

            if (indexToUse === undefined) {
              indexToUse = index;
            } else if (indexToUse !== index) {
              // another index has already been used
              result.addError(new FieldsFromMultipleIndicesError(sortingField));
            }
          }));
    });

  return result;
}

module.exports = validateIndexedProperties;
