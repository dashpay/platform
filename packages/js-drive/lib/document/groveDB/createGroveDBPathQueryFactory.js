const getPropertyDefinitionByPath = require('@dashevo/dpp/lib/dataContract/getPropertyDefinitionByPath');
const DataContractStoreRepository = require('../../dataContract/DataContractStoreRepository');
const createDocumentTypeTreePath = require('./createDocumentTreePath');

/**
 * @param {findAppropriateIndex} findAppropriateIndex
 * @param {sortWhereClausesAccordingToIndex} sortWhereClausesAccordingToIndex
 * @return {createGroveDBPathQuery}
 */
function createGroveDBPathQueryFactory(
  findAppropriateIndex,
  sortWhereClausesAccordingToIndex,
) {
  /**
   * @typedef {createGroveDBPathQuery}
   * @param {DataContract} dataContract
   * @param {string} documentType
   * @param [query]
   * @param [query.where]
   * @param [query.limit]
   * @param [query.startAt]
   * @param [query.startAfter]
   * @param [query.orderBy]
   *
   * @return {PathQuery}
   */
  function createGroveDBPathQuery(dataContract, documentType, query) {
    const documentSchema = dataContract.getDocumentSchema(documentType);

    const documentTypeTreePath = createDocumentTypeTreePath(dataContract, documentType);

    // Documents query
    const documentsQuery = new Query();

    documentsQuery.insertAllKeys();

    let path = documentTypeTreePath;
    let rangeQuery = new Query();
    let subQuery;
    let subQueryKey;

    let currentIndex;
    let whereClauses;

    // Range query
    if (query.where) {
      currentIndex = findAppropriateIndex(query.where, documentSchema);

      whereClauses = sortWhereClausesAccordingToIndex(query.where, currentIndex);

      const lastClause = whereClauses[whereClauses.length - 1];

      const [lastPropertyName, lastOperator, lastValue] = lastClause;

      const lastPropertyDefinition = getPropertyDefinitionByPath(
        documentSchema,
        lastPropertyName,
      );

      // eslint-disable-next-line default-case
      switch (lastOperator) {
        case 'in': {
          lastValue.forEach((item, i) => {
            let arrayPropertyDefinition;
            if (lastPropertyDefinition.items) {
              arrayPropertyDefinition = lastPropertyDefinition.items;
            } else {
              arrayPropertyDefinition = lastPropertyDefinition.prefixItems[i];
            }

            const encodedValue = this.encodeDocumentPropertyValue(
              item,
              arrayPropertyDefinition,
            );

            query.insertKey(encodedValue);
          });

          whereClauses.pop();

          break;
        }
        case 'startsWith': {
          const encodedValue = this.encodeDocumentPropertyValue(
            lastValue,
            lastPropertyDefinition,
          );

          query.inserKeyPrefix(encodedValue);

          whereClauses.pop();

          break;
        }
        case '<':
        case '<=':
        case '>':
        case '>=': {
          const encodedValue = this.encodeDocumentPropertyValue(
            lastValue,
            lastPropertyDefinition,
          );

          let leftBound;
          let leftBoundInclusive = false;
          let rightBound;
          let rightBoundInclusive = false;

          if (lastOperator.startsWith('<')) {
            leftBound = encodedValue;

            if (lastOperator.includes('=')) {
              leftBoundInclusive = true;
            }
          }

          if (lastOperator.startsWith('>')) {
            rightBound = encodedValue;

            if (lastOperator.includes('=')) {
              rightBoundInclusive = true;
            }
          }

          // Two bounds range
          const nextClause = whereClauses[whereClauses.length - 2];
          if (nextClause && (nextClause.includes('<') || nextClause.includes('>'))) {
            const [, previousOperator, previousValue] = nextClause;

            const nextEncodedValue = this.encodeDocumentPropertyValue(
              previousValue,
              lastPropertyDefinition,
            );

            if (previousOperator.startsWith('<')) {
              leftBound = nextEncodedValue;

              if (previousOperator.includes('=')) {
                leftBoundInclusive = true;
              }
            }

            if (previousOperator.startsWith('>')) {
              rightBound = nextEncodedValue;

              if (previousOperator.includes('=')) {
                rightBoundInclusive = true;
              }
            }

            whereClauses.pop();
          }

          // Create a range
          if (leftBound && rightBound) {
            if (leftBoundInclusive && rightBoundInclusive) {
              // RangeInclusive
              // insert_range_inclusive
            } else if (leftBoundInclusive) {
              // RangeFromInclusive
              // insert_range_from_inclusive
            } else if (rightBoundInclusive) {
              // RangeToInclusive
              // insert_range_to_inclusive
            } else {
              // Range
              // insert_range
            }
          } else if (leftBound) {
            // RangeFrom
            // insert_range_from
          } else if (rightBound) {
            // RangeTo
            // insert_range_to
          }

          whereClauses.pop();
        }
      }

      const pathSegments = whereClauses.map(([propertyName, , propertyValue]) => {
        const propertyDefinition = getPropertyDefinitionByPath(documentSchema, propertyName);

        const encodedValue = this.encodeDocumentPropertyValue(propertyValue, propertyDefinition);

        return [
          Buffer.from(propertyName),
          encodedValue,
        ];
      });

      path = path.concat(pathSegments.flat());

      subQuery = documentsQuery;
      subQueryKey = DataContractStoreRepository.DOCUMENTS_TREE_KEY;
    } else {
      // Query all documents for specified document type
      path.push(DataContractStoreRepository.DOCUMENTS_TREE_KEY);
      rangeQuery = documentsQuery;
    }

    // Apply sorting
    let order = 'asc';
    if (query.orderBy) {
      [[, order]] = query.orderBy;
    } else if (query.where.length) {
      // Get default for last clause according to index definition
      const lastIndexedProperty = currentIndex.properties[currentIndex.properties.length - 1];

      [order] = Object.values(lastIndexedProperty);
    }

    // Apply limit and offset
    // TODO: Allow startAt and startAfter for documentId
    let offset;
    if (query.startAt && query.startAt > 1) {
      offset = query.startAt - 1;
    }

    if (query.startAfter) {
      offset = query.startAfter;
    }

    const limit = query.limit || 100;

    const sizedQuery = new SizedQuery(
      rangeQuery,
      limit,
      offset,
      order === 'asc',
    );

    // Create PathQuery
    return new PathQuery(
      path,
      sizedQuery,
      subQuery,
      subQueryKey,
    );
  }

  return createGroveDBPathQuery;
}

module.exports = createGroveDBPathQueryFactory;
