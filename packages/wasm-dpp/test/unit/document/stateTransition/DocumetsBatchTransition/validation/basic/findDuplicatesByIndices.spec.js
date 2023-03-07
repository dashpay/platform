const DocumentJs = require('@dashevo/dpp/lib/document/Document');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const { generate: generateEntropy } = require('@dashevo/dpp/lib/util/entropyGenerator');
const { default: loadWasmDpp } = require('../../../../../../../dist');

let DataContract;
let Identifier;
let findDuplicatesByIndices;

describe('findDuplicatesByIndices', () => {
  let documents;
  let contractJs;
  let contract;
  let documentTransitions;

  beforeEach(async () => {
    ({
      DataContract,
      findDuplicatesByIndices,
      Identifier,
    } = await loadWasmDpp());
    contractJs = getDataContractFixture();
    contractJs.setDocumentSchema('nonUniqueIndexDocument', {
      indices: [
        {
          name: 'lastName',
          properties: [
            { lastName: 'asc' },
          ],
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

    contractJs.setDocumentSchema('singleDocument', {
      indices: [
        {
          name: 'ownerIdLastName',
          properties: [
            { $ownerId: 'asc' },
            { lastName: 'asc' },
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
      required: ['lastName'],
      additionalProperties: false,
    });

    contract = DataContract.fromBuffer(contractJs.toBuffer());

    documents = getDocumentsFixture(contractJs);
    documents.forEach((doc) => {
      // eslint-disable-next-line no-param-reassign
      doc.dataContract = contractJs;
      // eslint-disable-next-line no-param-reassign
      doc.dataContractId = contractJs.getId();
    });

    const [, , , william] = documents;

    let document = new DocumentJs({
      ...william.toObject(),
      $type: 'nonUniqueIndexDocument',
      $entropy: generateEntropy(),
    }, contractJs);

    document.setEntropy(generateEntropy());

    documents.push(document);

    document = new DocumentJs({
      ...william.toObject(),
      $type: 'singleDocument',
      $entropy: generateEntropy(),
    }, contractJs);

    document.setEntropy(generateEntropy());

    documents.push(document);

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((t) => t.toObject());
  });

  it('should return duplicate documents if they are present - Rust', () => {
    const [, , , , leon] = documents;

    leon.set('lastName', 'Birkin');
    const ownerId = Identifier.from(leon.ownerId);

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    }).map((t) => t.toObject());

    const duplicates = findDuplicatesByIndices(documentTransitions, contract, ownerId);

    expect(duplicates.length).to.equal(2);
    expect(duplicates).to.have.deep.members(
      [
        documentTransitions[3],
        documentTransitions[4],
      ],
    );
  });

  it('should return an empty array of there are no duplicates - Rust', () => {
    const [, , , , leon] = documents;
    const ownerId = Identifier.from(leon.ownerId);
    const duplicates = findDuplicatesByIndices(documentTransitions, contract, ownerId);

    expect(duplicates.length).to.equal(0);
  });
});
