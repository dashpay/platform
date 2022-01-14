const getPropertyDefinitionByPath = require('@dashevo/dpp/lib/dataContract/getPropertyDefinitionByPath');
const DataContractStoreRepository = require('../../dataContract/DataContractStoreRepository');
const createDocumentTypeTreePath = require('./createDocumentTreePath');

function createGroveDBPathQuery(dataContract, documentType, query) {
  const documentSchema = dataContract.getDocumentSchema(documentType);

  const documentTypeTreePath = createDocumentTypeTreePath(dataContract, documentType);

  // TODO: Allow startAt and startAfter for documentId
  let offset;
  if (query.startAt && query.startAt > 1) {
    offset = query.startAt - 1;
  }

  if (query.startAfter) {
    offset = query.startAfter;
  }

  const limit = query.limit || 100;

  const queryPaths = [];

  let staticPath = documentTypeTreePath;

  if (query.where) {
    let skipNextClause = false;

    query.where.forEach(([propertyName, operator, value], clauseIndex) => {
      if (skipNextClause) {
        return;
      }

      const propertyDefinition = getPropertyDefinitionByPath(documentSchema, propertyName);

      staticPath.push(propertyName);

      switch (operator) {
        case '==': {
          const encodedValue = this.encodeDocumentPropertyValue(value, propertyDefinition);

          staticPath.push(encodedValue);

          return;
        }
        case 'in': {
          const query = new Query();

          value.forEach((item, i) => {
            let arrayPropertyDefinition;
            if (propertyDefinition.items) {
              arrayPropertyDefinition = propertyDefinition.items;
            } else {
              arrayPropertyDefinition = propertyDefinition.prefixItems[i];
            }

            const encodedValue = this.encodeDocumentPropertyValue(item, arrayPropertyDefinition);
            query.insertKey(encodedValue);
          });

          const pathQuery = new PathQuery(staticPath, query);

          queryPaths.push(pathQuery);
          staticPath = [];
        }
        case 'startsWith': {
          throw new Error('Not implemented');
        }
        case '<':
        case '<=':
        case '>':
        case '>=': {
          const query = new Query();

          const encodedValue = this.encodeDocumentPropertyValue(value, propertyDefinition);

          let leftBound;
          let leftBoundInclusive = false;
          let rightBound;
          let rightBoundInclusive = false;

          if (operator.startsWith('<')) {
            leftBound = encodedValue;

            if (operator.includes('=')) {
              leftBoundInclusive = true;
            }
          }

          if (operator.startsWith('>')) {
            rightBound = encodedValue;

            if (operator.includes('=')) {
              rightBoundInclusive = true;
            }
          }

          // Two bounds range
          const nextClause = query.where[clauseIndex + 1];
          if (nextClause && (nextClause.includes('<') || nextClause.includes('>'))) {
            skipNextClause = true;

            const [, nextOperator, nextValue] = nextClause;

            const nextEncodedValue = this.encodeDocumentPropertyValue(nextValue, propertyDefinition);

            if (nextOperator.startsWith('<')) {
              leftBound = nextEncodedValue;

              if (nextOperator.includes('=')) {
                leftBoundInclusive = true;
              }
            }

            if (nextOperator.startsWith('>')) {
              rightBound = nextEncodedValue;

              if (nextOperator.includes('=')) {
                rightBoundInclusive = true;
              }
            }
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
        }
      }
    });
  }

  // Add "select all document Ids" path query
  {
    const documentIdsTreePath = staticPath.concat([DataContractStoreRepository.DOCUMENTS_TREE_KEY]);

    const query = new Query();

    query.insertAll();

    pathQueries.push(
      new PathQuery(documentIdsTreePath, query),
    );
  }

  return pathQueries;
}

module.exports = createGroveDBPathQuery;
