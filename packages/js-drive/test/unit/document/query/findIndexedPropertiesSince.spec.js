const findIndexedPropertiesSince = require('../../../../lib/document/query/findIndexedPropertiesSince');

describe('findIndexedPropertiesSince', () => {
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

  it('should return all matching indexed property lists since specific property', () => {
    const result = findIndexedPropertiesSince('a', 2, documentSchema);

    expect(result).to.have.deep.members([
      ['a', 'd'], ['a', 'd'],
    ]);
  });
});
