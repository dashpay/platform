const findAppropriateIndex = require('../../../../lib/document/query/findAppropriateIndex');

describe('findAppropriateIndex', () => {
  let whereClauses;
  let documentSchema;

  beforeEach(() => {
    documentSchema = {
      indices: [
        {
          properties: [{ a: 'asc' }],
          name: 'a',
        },
        {
          properties: [{ b: 'desc' }],
          name: 'b',
        },
        {
          properties: [{ b: 'desc' }, { a: 'desc' }],
          name: 'ba',
        },
        {
          properties: [{ b: 'desc' }, { a: 'desc' }, { d: 'desc' }],
          name: 'bad',
        },
      ],
    };
  });

  it('should find appropriate index', () => {
    whereClauses = [
      ['a', '==', 1],
      ['b', '==', 2],
    ];

    let result = findAppropriateIndex(whereClauses, documentSchema);

    expect(result).to.be.deep.equal({ properties: [{ b: 'desc' }, { a: 'desc' }], name: 'ba' });

    whereClauses = [
      ['a', '==', 1],
    ];

    result = findAppropriateIndex(whereClauses, documentSchema);

    expect(result).to.be.deep.equal({ properties: [{ a: 'asc' }], name: 'a' });
  });

  it('should find appropriate index with system field', () => {
    whereClauses = [
      ['$id', '==', 2],
    ];

    const result = findAppropriateIndex(whereClauses, documentSchema);

    expect(result).to.be.deep.equal({
      properties: [
        {
          $id: 'asc',
        },
      ],
      unique: true,
    });
  });

  it('should return empty object', () => {
    whereClauses = [
      ['c', '==', 1],
    ];

    let result = findAppropriateIndex(whereClauses, documentSchema);

    expect(result).to.be.deep.equal(undefined);

    whereClauses = [
      ['d', '==', 1],
    ];

    result = findAppropriateIndex(whereClauses, documentSchema);

    expect(result).to.be.deep.equal(undefined);

    whereClauses = [
      ['b', '==', 1],
      ['d', '==', 1],
    ];

    result = findAppropriateIndex(whereClauses, documentSchema);

    expect(result).to.be.deep.equal(undefined);
  });
});
