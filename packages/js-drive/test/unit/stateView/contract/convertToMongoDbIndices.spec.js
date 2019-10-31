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
    ];
  });

  it('should return converted data', async () => {
    const convertedIndices = convertToMongoDbIndices(indicesFixture);

    expect(convertedIndices).to.deep.equal([{
      key: {
        $userId: 1,
        firstName: -1,
      },
      unique: true,
      name: '$userId_firstName',
    }, {
      key: {
        $userId: 1,
        lastName: -1,
      },
      unique: false,
      name: '$userId_lastName',
    }]);
  });
});
