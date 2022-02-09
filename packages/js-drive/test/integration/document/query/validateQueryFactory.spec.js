const validateQueryFactory = require('../../../../lib/document/query/validateQueryFactory');

const findConflictingConditions = require('../../../../lib/document/query/findConflictingConditions');
const findAppropriateIndex = require('../../../../lib/document/query/findAppropriateIndex');
const sortWhereClausesAccordingToIndex = require('../../../../lib/document/query/sortWhereClausesAccordingToIndex');
const findThreesomeOfIndexedProperties = require('../../../../lib/document/query/findThreesomeOfIndexedProperties');
const findIndexedPropertiesSince = require('../../../../lib/document/query/findIndexedPropertiesSince');
const WhereConditionPropertiesNumberError = require('../../../../lib/document/query/errors/WhereConditionPropertiesNumberError');
const InOperatorAllowedOnlyForLastTwoIndexedPropertiesError = require('../../../../lib/document/query/errors/InOperatorAllowedOnlyForLastTwoIndexedPropertiesError');
const ConflictingConditionsError = require('../../../../lib/document/query/errors/ConflictingConditionsError');
const NotIndexedPropertiesInWhereConditionsError = require('../../../../lib/document/query/errors/NotIndexedPropertiesInWhereConditionsError');
const RangeOperatorAllowedOnlyForLastTwoWhereConditionsError = require('../../../../lib/document/query/errors/RangeOperatorAllowedOnlyForLastTwoWhereConditionsError');
const MultipleRangeOperatorsError = require('../../../../lib/document/query/errors/MultipleRangeOperatorsError');
const RangePropertyDoesNotHaveOrderByError = require('../../../../lib/document/query/errors/RangePropertyDoesNotHaveOrderByError');
const InvalidOrderByPropertiesOrderError = require('../../../../lib/document/query/errors/InvalidOrderByPropertiesOrderError');

describe('validateQueryFactory', () => {
  const validQueries = [
    {
      where: [
        ['a', '==', 1],
        ['b', '==', 2],
        ['c', '==', 3],
        ['d', 'in', [1, 2]],
      ],
      orderBy: [
        ['d', 'desc'],
        ['e', 'asc'],
      ],
    },
    {
      where: [
        ['a', '==', 1],
        ['b', '==', 2],
        ['c', '==', 3],
        ['d', 'in', [1, 2]],
        ['e', '>', 3],
      ],
      orderBy: [
        ['d', 'desc'],
        ['e', 'asc'],
      ],
    },
    {
      where: [
        ['firstName', '>', 'Chris'],
        ['firstName', '<=', 'Noellyn'],
      ],
      orderBy: [
        ['firstName', 'asc'],
      ],
    },
  ];

  const invalidQueries = [
    {
      query: {
        where: [
          ['a', '==', 1],
          ['b', '==', 2],
        ],
      },
      errorClass: WhereConditionPropertiesNumberError,
    },
    {
      query: {
        where: [
          ['a', '==', 1],
          ['b', '==', 2],
          ['c', 'in', [1, 2]],
        ],
        orderBy: [
          ['c', 'desc'],
        ],
      },
      errorClass: InOperatorAllowedOnlyForLastTwoIndexedPropertiesError,
    },
    {
      query: {
        where: [
          ['a', '==', 1],
          ['b', '==', 2],
          ['b', 'in', [1, 2]],
        ],
        orderBy: [
          ['b', 'desc'],
        ],
      },
      errorClass: ConflictingConditionsError,
    },
    {
      query: {
        where: [
          ['z', '==', 1],
        ],
      },
      errorClass: NotIndexedPropertiesInWhereConditionsError,
    },
    {
      query: {
        where: [
          ['a', '==', 1],
          ['b', '==', 2],
          ['c', '>', 3],
          ['d', 'in', [1, 2]],
          ['e', '>', 3],
        ],
      },
      errorClass: RangeOperatorAllowedOnlyForLastTwoWhereConditionsError,
    },
    {
      query: {
        where: [
          ['a', '==', 1],
          ['b', '==', 2],
          ['c', '>', 3],
          ['d', '>', 3],
        ],
        orderBy: [
          ['c', 'asc'],
          ['d', 'desc'],
        ],
      },
      errorClass: MultipleRangeOperatorsError,
    },
    {
      query: {
        where: [
          ['a', '==', 3],
          ['b', '==', 2],
          ['c', '>', 1],
        ],
      },
      errorClass: RangePropertyDoesNotHaveOrderByError,
    },
    {
      query: {
        where: [
          ['a', '==', 3],
          ['b', '==', 2],
          ['c', '==', 3],
          ['d', 'in', [1, 2]],
          ['e', '<', 1],
        ],
        orderBy: [
          ['e', 'asc'],
          ['d', 'asc'],
        ],
      },
      errorClass: InvalidOrderByPropertiesOrderError,
    },
  ];

  let documentSchema;
  let validateQuery;

  beforeEach(() => {
    documentSchema = {
      indices: [
        {
          name: 'one',
          properties: [
            { firstName: 'asc' },
          ],
        },
        {
          name: 'two',
          properties: [
            { a: 'asc' },
            { b: 'asc' },
            { c: 'asc' },
            { d: 'asc' },
            { e: 'asc' },
          ],
        },
      ],
    };

    validateQuery = validateQueryFactory(
      findConflictingConditions,
      findAppropriateIndex,
      sortWhereClausesAccordingToIndex,
      findThreesomeOfIndexedProperties,
      findIndexedPropertiesSince,
    );
  });

  validQueries.forEach((query) => {
    it(`should return valid result for query "${JSON.stringify(query)}"`, () => {
      const result = validateQuery(query, documentSchema);

      expect(result.isValid()).to.be.true();
    });
  });

  invalidQueries.forEach(({ query, errorClass }) => {
    it(`should return invalid result with "${errorClass}" error for query "${JSON.stringify(query)}"`, () => {
      const result = validateQuery(query, documentSchema);

      expect(result.isValid()).to.be.false();

      const resultError = result.getErrors()[0];

      expect(resultError).to.be.an.instanceof(errorClass);
    });
  });
});
