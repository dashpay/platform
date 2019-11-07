/**
 * Validate query to search by only indexed fields
 *
 * @typedef findNotIndexedFields
 * @param {{field, direction}[][]} dataContractIndexFields
 * @param {[string, string, string][]} conditions
 * @returns {Array}
 */
function findNotIndexedFields(dataContractIndexFields, conditions) {
  // convert conditions to better format
  const queryFields = conditions
    .reduce((fields, [field, operator, elementMatchValue]) => {
      let fieldsToAdd = [];

      if (operator === 'elementMatch') {
        fieldsToAdd = elementMatchValue.map(([item]) => `${field}.${item}`);
      } else {
        fieldsToAdd = [field];
      }

      return fields.concat(fieldsToAdd);
    }, []);

  // validate fields
  return queryFields
    .filter((field) => {
      const fieldHasIndex = dataContractIndexFields
      // find our field in indices
        .find(index => index
          // search through compound index
          .find((element, i) => {
            const [indexField] = Object.keys(element);
            if (indexField !== field) {
              return false;
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
}

module.exports = findNotIndexedFields;
