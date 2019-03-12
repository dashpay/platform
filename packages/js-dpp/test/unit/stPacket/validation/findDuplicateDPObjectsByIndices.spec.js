const findDuplicateDPObjectsByIndices = require('../../../../lib/stPacket/validation/findDuplicateDPObjectsByIndices');

const getDPContractFixture = require('../../../../lib/test/fixtures/getDPContractFixture');
const getDPObjectsFixture = require('../../../../lib/test/fixtures/getDPObjectsFixture');

describe('findDuplicateDPObjectsByIndices', () => {
  let rawDPObjects;
  let dpContract;

  beforeEach(() => {
    rawDPObjects = getDPObjectsFixture().map(o => o.toJSON());

    dpContract = getDPContractFixture();
    dpContract.setDPObjectSchema('nonUniqueIndexObject', {
      indices: [
        {
          properties: {
            $userId: 'asc',
            lastName: 'asc',
          },
          unique: false,
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
      required: ['lastName'],
      additionalProperties: false,
    });

    dpContract.setDPObjectSchema('singleObject', {
      indices: [
        {
          properties: {
            $userId: 'asc',
            lastName: 'asc',
          },
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
      required: ['lastName'],
      additionalProperties: false,
    });

    const [, , , william] = rawDPObjects;

    rawDPObjects.push(Object.assign({}, william, {
      $type: 'nonUniqueIndexObject',
    }));

    rawDPObjects.push(Object.assign({}, william, {
      $type: 'singleObject',
    }));
  });

  it('should return duplicate objects if they are present', () => {
    const [, , , william, leon] = rawDPObjects;
    leon.lastName = 'Birkin';

    const duplicates = findDuplicateDPObjectsByIndices(rawDPObjects, dpContract);
    expect(duplicates).to.deep.equal(
      [
        leon,
        william,
      ],
    );
  });

  it('should return an empty array of there are no duplicates', () => {
    const duplicates = findDuplicateDPObjectsByIndices(rawDPObjects, dpContract);
    expect(duplicates.length).to.equal(0);
  });
});
