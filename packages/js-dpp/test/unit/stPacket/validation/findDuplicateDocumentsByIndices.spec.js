const findDuplicateDocumentsByIndices = require('../../../../lib/stPacket/validation/findDuplicateDocumentsByIndices');

const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');
const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');

describe.skip('findDuplicateDocumentsByIndices', () => {
  let rawDocuments;
  let contract;

  beforeEach(() => {
    rawDocuments = getDocumentsFixture().map(o => o.toJSON());

    contract = getContractFixture();
    contract.setDocumentSchema('nonUniqueIndexDocument', {
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

    contract.setDocumentSchema('singleDocument', {
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

    const [, , , william] = rawDocuments;

    rawDocuments.push(Object.assign({}, william, {
      $type: 'nonUniqueIndexDocument',
    }));

    rawDocuments.push(Object.assign({}, william, {
      $type: 'singleDocument',
    }));
  });

  it('should return duplicate documents if they are present', () => {
    const [, , , william, leon] = rawDocuments;
    leon.lastName = 'Birkin';

    const duplicates = findDuplicateDocumentsByIndices(rawDocuments, contract);
    expect(duplicates).to.deep.equal(
      [
        leon,
        william,
      ],
    );
  });

  it('should return an empty array of there are no duplicates', () => {
    const duplicates = findDuplicateDocumentsByIndices(rawDocuments, contract);
    expect(duplicates.length).to.equal(0);
  });
});
