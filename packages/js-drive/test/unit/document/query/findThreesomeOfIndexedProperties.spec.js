const findThreesomeOfIndexedProperties = require('../../../../lib/document/query/findThreesomeOfIndexedProperties');

describe('findThreesomeOfIndexedProperties', () => {
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
        {
          properties: [{ b: 'desc' }, { a: 'desc' }, { d: 'desc' }, { e: 'asc' }],
          name: 'bade',
        },
      ],
    };
  });

  it('should return all threesomes and twosome with a property from all the indices', () => {
    const result = findThreesomeOfIndexedProperties('a', documentSchema);

    expect(result).to.have.deep.members([
      ['a'], ['a'], ['a', 'd'], ['a', 'd'],
    ]);
  });
});
