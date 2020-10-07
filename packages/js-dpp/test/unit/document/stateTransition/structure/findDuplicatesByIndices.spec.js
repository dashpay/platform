const Document = require('../../../../../lib/document/Document');

const findDuplicateDocumentsByIndices = require('../../../../../lib/document/stateTransition/validation/structure/findDuplicatesByIndices');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

const entropy = require('../../../../../lib/util/entropy');

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

    documents = getDocumentsFixture(contract);
    documents.forEach((doc) => {
      // eslint-disable-next-line no-param-reassign
      doc.dataContract = contract;
      // eslint-disable-next-line no-param-reassign
      doc.dataContractId = contract.getId();
    });

    const [, , , william] = documents;

    let document = new Document({
      ...william.toObject(),
      $type: 'nonUniqueIndexDocument',
      $entropy: entropy.generate(),
    }, contract);

    document.setEntropy(entropy.generate());

    documents.push(document);

    document = new Document({
      ...william.toObject(),
      $type: 'singleDocument',
      $entropy: entropy.generate(),
    }, contract);

    document.setEntropy(entropy.generate());

    documents.push(document);

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((t) => t.toObject());
  });

  it('should return duplicate documents if they are present', () => {
    const [, , , , leon] = documents;

    leon.set('lastName', 'Birkin');

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((t) => t.toObject());

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
