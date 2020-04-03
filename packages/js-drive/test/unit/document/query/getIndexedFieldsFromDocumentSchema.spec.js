const getIndexedFieldsFromDocumentSchema = require('../../../../lib/document/query/getIndexedFieldsFromDocumentSchema');

describe('getIndexedFieldsFromDocumentSchema', () => {
  let documentSchema;

  beforeEach(() => {
    documentSchema = {
      indices: [
        {
          properties: [
            { middleName: 'asc' },
          ],
        },
        {
          properties: [
            { $ownerId: 'asc' },
            { firstName: 'desc' },
          ],
          unique: true,
        },
        {
          properties: [
            { $ownerId: 'asc' },
            { lastName: 'desc' },
          ],
          unique: true,
        },
      ],
      properties: {
        firstName: {
          type: 'string',
        },
        lastName: {
          type: 'string',
        },
      },
      required: ['firstName'],
      additionalProperties: false,
    };
  });

  it('should return indexed fields', () => {
    const result = getIndexedFieldsFromDocumentSchema(documentSchema);

    expect(result).to.be.an('array');
    expect(result).to.deep.equal([
      [{ middleName: 'asc' }],
      [{ $ownerId: 'asc' }, { firstName: 'desc' }],
      [{ $ownerId: 'asc' }, { lastName: 'desc' }],
      [{ $id: 'asc' }],
      [{ $id: 'desc' }],
    ]);
  });

  it('should return an array with system field if schema does not contain indices', async () => {
    delete documentSchema.indices;

    const result = getIndexedFieldsFromDocumentSchema(documentSchema);

    expect(result).to.be.an('array');
    expect(result).to.deep.equal([
      [{ $id: 'asc' }],
      [{ $id: 'desc' }],
    ]);
  });
});
