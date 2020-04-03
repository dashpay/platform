const convertToMongoDbIndices = require('../../../../lib/document/mongoDbRepository/convertToMongoDbIndices');

describe('convertToMongoDbIndices', () => {
  let indicesFixture;

  beforeEach(() => {
    indicesFixture = [
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
      },
      {
        properties: [
          { $id: 'asc' },
          { lastName: 'desc' },
        ],
      },
    ];
  });

  it('should return converted data', () => {
    const convertedIndices = convertToMongoDbIndices(indicesFixture);

    expect(convertedIndices).to.deep.equal([{
      key: {
        ownerId: 1,
        'data.firstName': -1,
      },
      unique: true,
      name: 'ownerId_data.firstName',
    }, {
      key: {
        ownerId: 1,
        'data.lastName': -1,
      },
      unique: false,
      name: 'ownerId_data.lastName',
    }, {
      key: {
        _id: 1,
        'data.lastName': -1,
      },
      unique: false,
      name: '_id_data.lastName',
    }]);
  });
});
