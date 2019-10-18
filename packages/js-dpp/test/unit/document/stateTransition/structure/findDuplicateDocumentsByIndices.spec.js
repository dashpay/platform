const Document = require('../../../../../lib/document/Document');

const findDuplicateDocumentsByIndices = require('../../../../../lib/document/stateTransition/validation/structure/findDuplicateDocumentsByIndices');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');

describe('findDuplicateDocumentsByIndices', () => {
  let documents;
  let contract;

  beforeEach(() => {
    contract = getDataContractFixture();
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

    documents = getDocumentsFixture();

    const [, , , william] = documents;

    documents.push(new Document({
      ...william.toJSON(),
      $type: 'nonUniqueIndexDocument',
    }));

    documents.push(new Document({
      ...william.toJSON(),
      $type: 'singleDocument',
    }));
  });

  it('should return duplicate documents if they are present', () => {
    const [, , , william, leon] = documents;

    leon.set('lastName', 'Birkin');

    const duplicates = findDuplicateDocumentsByIndices(documents, contract);
    expect(duplicates).to.deep.equal(
      [
        leon.toJSON(),
        william.toJSON(),
      ],
    );
  });

  it('should return an empty array of there are no duplicates', () => {
    const duplicates = findDuplicateDocumentsByIndices(documents, contract);

    expect(duplicates.length).to.equal(0);
  });
});
