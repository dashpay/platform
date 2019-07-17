const verifyDocumentsUniquenessByIndicesFactory = require('../../../../lib/stPacket/verification/verifyDocumentsUniquenessByIndicesFactory');

const STPacket = require('../../../../lib/stPacket/STPacket');

const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

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

  it('should return valid result if Documents have no unique indices');

  it('should return valid result if Document has unique indices and there are no duplicates', async () => {
    const [, , , william] = documents;

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        william.getType(),
        {
          where: [
            ['$userId', '==', userId],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([william.toJSON()]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        william.getType(),
        {
          where: [
            ['$userId', '==', userId],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .resolves([william.toJSON()]);

    const result = await verifyDocumentsUniquenessByIndices(stPacket, userId, contract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result if Document has unique indices and there are duplicates', async () => {
    const [, , , william, leon] = documents;

    const indicesDefinition = contract.getDocumentSchema(william.getType()).indices;

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        william.getType(),
        {
          where: [
            ['$userId', '==', userId],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([leon.toJSON()]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        william.getType(),
        {
          where: [
            ['$userId', '==', userId],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .resolves([leon.toJSON()]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        leon.getType(),
        {
          where: [
            ['$userId', '==', userId],
            ['firstName', '==', leon.get('firstName')],
          ],
        },
      )
      .resolves([william.toJSON()]);

    dataProviderMock.fetchDocuments
      .withArgs(
        stPacket.getContractId(),
        leon.getType(),
        {
          where: [
            ['$userId', '==', userId],
            ['lastName', '==', leon.get('lastName')],
          ],
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
