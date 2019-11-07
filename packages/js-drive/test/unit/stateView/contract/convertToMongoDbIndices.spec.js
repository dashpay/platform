const convertToMongoDbIndices = require('../../../../lib/stateView/contract/convertToMongoDbIndices');

describe('convertToMongoDbIndices', () => {
  let indicesFixture;

  beforeEach(() => {
    indicesFixture = [
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
        userId: 1,
        'data.firstName': -1,
      },
      unique: true,
      name: 'userId_data.firstName',
    }, {
      key: {
        userId: 1,
        'data.lastName': -1,
      },
      unique: false,
      name: 'userId_data.lastName',
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
