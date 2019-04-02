const verifyDocumentsUniquenessByIndicesFactory = require('../../../../lib/stPacket/verification/verifyDocumentsUniquenessByIndicesFactory');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

const DuplicateDocumentError = require('../../../../lib/errors/DuplicateDocumentError');

describe('verifyDocumentsUniquenessByIndices', () => {
  let fetchDocumentsByDocumentsMock;
  let dataProviderMock;
  let verifyDocumentsUniquenessByIndices;
  let stPacket;
  let documents;
  let contract;
  let userId;

  beforeEach(function beforeEach() {
    ({ userId } = getDocumentsFixture);

    documents = getDocumentsFixture();
    contract = getContractFixture();

    stPacket = new STPacket(contract.getId());
    stPacket.setDocuments(documents);

    fetchDocumentsByDocumentsMock = this.sinonSandbox.stub();

    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    dataProviderMock.fetchDocuments.resolves([]);

    verifyDocumentsUniquenessByIndices = verifyDocumentsUniquenessByIndicesFactory(
      fetchDocumentsByDocumentsMock,
      dataProviderMock,
    );
  });

  it('should return invalid result if Document has unique indices and there are duplicates', async () => {
    const [, , , william, leon] = documents;

    const indicesDefinition = contract.getDocumentSchema(william.getType()).indices;

    dataProviderMock.fetchDocuments.resolves([]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        william.getType(),
        {
          where: {
            userId,
            'document.firstName': william.get('firstName'),
            _id: { $ne: william.getId() },
          },
        },
      )
      .resolves([leon.toJSON()]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        william.getType(),
        {
          where: {
            userId,
            'document.lastName': william.get('lastName'),
            _id: { $ne: william.getId() },
          },
        },
      )
      .resolves([leon.toJSON()]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        leon.getType(),
        {
          where: {
            userId,
            'document.firstName': leon.get('firstName'),
            _id: { $ne: leon.getId() },
          },
        },
      )
      .resolves([william.toJSON()]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        leon.getType(),
        {
          where: {
            userId,
            'document.lastName': leon.get('lastName'),
            _id: { $ne: leon.getId() },
          },
        },
      )
      .resolves([william.toJSON()]);

    const result = await verifyDocumentsUniquenessByIndices(stPacket, userId, contract);

    expectValidationError(result, DuplicateDocumentError, 4);

    const errors = result.getErrors();

    expect(errors.map(e => e.getDocument())).to.have.deep.members([
      william,
      william,
      leon,
      leon,
    ]);

    expect(errors.map(e => e.getIndexDefinition())).to.have.deep.members([
      indicesDefinition[0],
      indicesDefinition[1],
      indicesDefinition[0],
      indicesDefinition[1],
    ]);
  });
});
