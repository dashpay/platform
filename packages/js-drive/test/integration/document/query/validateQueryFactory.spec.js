const validateQueryFactory = require('../../../../lib/document/query/validateQueryFactory');

const findConflictingConditions = require('../../../../lib/document/query/findConflictingConditions');
const findAppropriateIndex = require('../../../../lib/document/query/findAppropriateIndex');
const sortWhereClausesAccordingToIndex = require('../../../../lib/document/query/sortWhereClausesAccordingToIndex');
const findThreesomeOfIndexedProperties = require('../../../../lib/document/query/findThreesomeOfIndexedProperties');
const findIndexedPropertiesSince = require('../../../../lib/document/query/findIndexedPropertiesSince');

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
      error: new Error('"where" conditions should have not less than "number of indexed properties - 2" properties'),
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
      error: new Error('Invalid range clause with \'c\' and \'in\' operator. "in" operator are allowed only for the last two indexed properties'),
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
      error: new Error('Using multiple conditions (==, in) with a single field ("b") is not allowed'),
    },
    {
      query: {
        where: [
          ['z', '==', 1],
        ],
      },
      error: new Error('Properties in where conditions must be defined as a document index'),
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
      error: new Error('\'>\', \'<\', \'>=\', \'<=\' operators are allowed only in the last two where conditions'),
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
      error: new Error('Invalid range clause with \'d\' and \'>\' operator. Only one range operator is allowed'),
    },
    {
      query: {
        where: [
          ['a', '==', 3],
          ['b', '==', 2],
          ['c', '>', 1],
        ],
      },
      error: new Error('Invalid range clause with \'c\' and \'>\' operator. Range operator must be used with order by'),
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
      error: new Error('"orderBy" properties order does not match order in compound index'),
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

  invalidQueries.forEach(({ query, error }) => {
    it(`should return invalid result with error "${error.message}" for query "${JSON.stringify(query)}"`, () => {
      const result = validateQuery(query, documentSchema);

      expect(result.isValid()).to.be.false();

      const resultError = result.getErrors()[0];

      expect(resultError.message).to.equal(error.message);
    });
  });
});
