/**
 * @typedef {[string, string]} indexStructure
 */

/**
 * Check if conditions satisfy indices:
 * index fields are in the same order, that sorting fields
 * and have the same direction
 *
 * @param {indexStructure[]} indexedFields
 * @param {indexStructure[]} sortingFields
 * @returns {boolean}
 */
function areSortingFieldsIndexedInRightOrder(indexedFields, sortingFields) {
  if (sortingFields.length < indexedFields.length) {
    return false;
  }

  return indexedFields
    .every(([field, direction], i) => {
      const [conditionField, conditionDirection] = sortingFields[i];

      return field === conditionField && direction === conditionDirection;
    });
}

/**
 * check if we have all indexed fields in where condition
 * @param {indexStructure[]} compoundIndexBefore
 * @param {[string, string, string][]} whereConditions
 * @returns {boolean}
 */
function areIndexedFieldsPresentInWhereCondition(compoundIndexBefore, whereConditions) {
  // flat where conditions
  const compiledWhereConditions = whereConditions.reduce(
    (fields, [field, operator, elementMatchValue]) => {
      let fieldsToAdd = [];

      if (operator === 'elementMatch') {
        fieldsToAdd = elementMatchValue.map(([item]) => `${field}.${item}`);
      } else {
        fieldsToAdd = [field];
      }

      return fields.concat(fieldsToAdd);
    },
    [],
  );

  return compoundIndexBefore
    .every(([field]) => compiledWhereConditions
      .some(conditionField => field === conditionField));
}

/**
 * Validate query to sort by only indexed fields
 *
 * @typedef findNotIndexedOrderByFields
 * @param {{field, direction}[][]} dataContractIndexFields
 * @param {indexStructure[]} sortingFields
 * @param {[string, string, string][]} [whereConditions]
 * @returns {Array}
 */
function findNotIndexedOrderByFields(
  dataContractIndexFields,
  sortingFields,
  whereConditions = [],
) {
  // convert dataContractIndex index objects to array
  const dataContractIndexedFields = dataContractIndexFields.map(
    compoundIndex => compoundIndex.map((index) => {
      const [firstIndexedField] = Object.entries(index);

      return firstIndexedField;
    }),
  );

  // mongoDB can sort by two directions in single field indices. So we can add this directions.
  const additionalIndices = [];
  dataContractIndexedFields.forEach((compoundIndex) => {
    if (compoundIndex.length === 1) {
      const [field, direction] = compoundIndex[0];
      const newDirection = direction === 'asc' ? 'desc' : 'asc';
      const alreadyHasIndex = dataContractIndexedFields.some(
        (indexArray) => {
          const [oldField, oldDirection] = indexArray[0];

          return (
            indexArray.length === 1
            && oldField === field
            && oldDirection === newDirection
          );
        },
      );

      if (!alreadyHasIndex) {
        additionalIndices.push([[field, newDirection]]);
      }
    }
  });

  const dataContractIndexedFieldsWithBothDirections = dataContractIndexedFields.concat(
    additionalIndices,
  );

  /**
   * mongoDB can't combine indices on sorting operations
   * so we leave only indices that contain all sorting fields
   *
   * @type {indexStructure[][]}
   */
  const indexedSortingFields = dataContractIndexedFieldsWithBothDirections.filter(
    compoundIndex => sortingFields
      .every(([field, direction]) => compoundIndex
        .some(
          ([indexField, indexDirection]) => field === indexField && direction === indexDirection,
        )),
  );

  // leave only only fields that not pass out validation
  return sortingFields
    .filter(([sortingField], i) => (
      !indexedSortingFields
        // iterate over indices
        .find(index => index
          // iterate over index fields
          .find(([indexField], j) => {
            // skip index field if not equal to sorting field
            if (indexField !== sortingField) {
              return false;
            }

            // part of compound index before our sorting field
            const compoundIndexBefore = index.slice(0, j);

            // part of compound index after our sorting field
            const compoundIndexAfter = index.slice(j, index.length);

            // we need to check sorting fields order too
            // so we need to compare two parts of it - before current field and after
            const sortingFieldsBefore = sortingFields.slice(0, i);
            const sortingFieldsAfter = sortingFields.slice(i, sortingFields.length);

            // check that all index fields of this part are inside
            // where or sorting conditions with right order
            const compoundFieldsBeforeIsOk = areIndexedFieldsPresentInWhereCondition(
              compoundIndexBefore, whereConditions,
            ) || areSortingFieldsIndexedInRightOrder(compoundIndexBefore, sortingFieldsBefore);

            // check that all index fields of this part are inside
            // sorting conditions and have right order
            const compoundFieldsAfterIsOk = areSortingFieldsIndexedInRightOrder(
              compoundIndexAfter.slice(0, sortingFieldsAfter.length),
              sortingFieldsAfter,
            );

            return compoundFieldsBeforeIsOk && compoundFieldsAfterIsOk;
          }))
    ))
    // return only fields
    .map(([field]) => field);
}

module.exports = findNotIndexedOrderByFields;
