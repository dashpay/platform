const sortWhereClausesAccordingToIndex = require('../../../../lib/document/query/sortWhereClausesAccordingToIndex');

describe('sortWhereClausesAccordingToIndex', () => {
  let whereClauses;
  let indexDefinition;

  beforeEach(() => {
    whereClauses = [
      ['c', '==', 1],
      ['a', '==', 1],
      ['b', '==', 2],
    ];
    indexDefinition = {
      properties: [{ b: 'desc' }, { a: 'desc' }, { c: 'asc' }],
    };
  });

  it('should sort where clauses according to index', () => {
    let result = sortWhereClausesAccordingToIndex(whereClauses, indexDefinition);

    expect(result).to.be.deep.equal([['b', '==', 2], ['a', '==', 1], ['c', '==', 1]]);

    indexDefinition = {
      properties: [{ b: 'desc' }, { a: 'desc' }, { c: 'asc' }, { d: 'desc' }],
    };

    result = sortWhereClausesAccordingToIndex(whereClauses, indexDefinition);

    expect(result).to.be.deep.equal([['b', '==', 2], ['a', '==', 1], ['c', '==', 1]]);

    whereClauses = [
      ['b', '==', 1],
    ];

    result = sortWhereClausesAccordingToIndex(whereClauses, indexDefinition);

    expect(result).to.be.deep.equal([['b', '==', 1]]);

    indexDefinition = {
      properties: [{ b: 'desc' }],
    };

    expect(result).to.be.deep.equal([['b', '==', 1]]);
  });
});
