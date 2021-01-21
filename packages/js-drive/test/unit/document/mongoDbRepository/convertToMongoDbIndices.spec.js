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
    }, {
      key: {
        ownerId: 1,
        'data.lastName': -1,
      },
      unique: false,
    }, {
      key: {
        _id: 1,
        'data.lastName': -1,
      },
      unique: false,
    }]);
  });
});
