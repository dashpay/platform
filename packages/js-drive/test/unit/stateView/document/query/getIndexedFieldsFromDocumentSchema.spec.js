const getIndexedFieldsFromDocumentSchema = require('../../../../../lib/stateView/document/query/getIndexedFieldsFromDocumentSchema');

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
            { $userId: 'asc' },
            { firstName: 'desc' },
          ],
          unique: true,
        },
        {
          properties: [
            { $userId: 'asc' },
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
      [{ $userId: 'asc' }, { firstName: 'desc' }],
      [{ $userId: 'asc' }, { lastName: 'desc' }],
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
