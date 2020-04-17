const Document = require('../../../../../lib/document/Document');

const findDuplicateDocumentsByIndices = require('../../../../../lib/document/stateTransition/validation/structure/findDuplicatesByIndices');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

describe('findDuplicatesByIndices', () => {
  let documents;
  let contract;
  let documentTransitions;

  beforeEach(() => {
    contract = getDataContractFixture();
    contract.setDocumentSchema('nonUniqueIndexDocument', {
      indices: [
        {
          properties: {
            $ownerId: 'asc',
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
            $ownerId: 'asc',
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

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((t) => t.toJSON());
  });

  it('should return duplicate documents if they are present', () => {
    const [, , , , leon] = documents;

    leon.set('lastName', 'Birkin');

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((t) => t.toJSON());

    const duplicates = findDuplicateDocumentsByIndices(documentTransitions, contract);
    expect(duplicates).to.have.deep.members(
      [
        documentTransitions[3],
        documentTransitions[4],
      ],
    );
  });

  it('should return an empty array of there are no duplicates', () => {
    const duplicates = findDuplicateDocumentsByIndices(documentTransitions, contract);

    expect(duplicates.length).to.equal(0);
  });
});
