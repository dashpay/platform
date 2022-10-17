const verifyDocumentsUniquenessByIndicesFactory = require('../../../../../../../lib/document/stateTransition/DocumentsBatchTransition/validation/state/validateDocumentsUniquenessByIndicesFactory');

const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const DuplicateUniqueIndexError = require('../../../../../../../lib/errors/consensus/state/document/DuplicateUniqueIndexError');
const StateTransitionExecutionContext = require('../../../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('validateDocumentsUniquenessByIndices', () => {
  let stateRepositoryMock;
  let validateDocumentsUniquenessByIndices;
  let documents;
  let documentTransitions;
  let dataContract;
  let ownerId;
  let executionContext;

  beforeEach(function beforeEach() {
    ({ ownerId } = getDocumentsFixture);

    documents = getDocumentsFixture(dataContract);
    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });
    dataContract = getContractFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDocuments.resolves([]);

    executionContext = new StateTransitionExecutionContext();

    validateDocumentsUniquenessByIndices = verifyDocumentsUniquenessByIndicesFactory(
      stateRepositoryMock,
    );
  });

  it('should return valid result if Documents have no unique indices', async () => {
    const [niceDocument] = documents;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    const result = await validateDocumentsUniquenessByIndices(
      ownerId,
      noIndexDocumentTransitions,
      dataContract,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMock.fetchDocuments).to.have.not.been.called();
  });

  it('should return valid result if Document has unique indices and there are no duplicates', async () => {
    const [, , , william] = documents;

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId().toBuffer(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([william]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId().toBuffer(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .resolves([william]);

    const result = await validateDocumentsUniquenessByIndices(
      ownerId,
      documentTransitions,
      dataContract,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result if Document has unique indices and there are duplicates', async () => {
    const [, , , william, leon] = documents;

    const indicesDefinition = dataContract.getDocumentSchema(william.getType()).indices;

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([leon]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .resolves([leon]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId(),
        leon.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
            ['firstName', '==', leon.get('firstName')],
          ],
        },
      )
      .resolves([william]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId(),
        leon.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
            ['lastName', '==', leon.get('lastName')],
          ],
        },
      )
      .resolves([william]);

    const result = await validateDocumentsUniquenessByIndices(
      ownerId,
      documentTransitions,
      dataContract,
      executionContext,
    );

    expectValidationError(result, DuplicateUniqueIndexError, 4);

    const errors = result.getErrors();

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4009);

    expect(errors.map((e) => e.getDocumentId())).to.have.deep.members([
      documentTransitions[3].getId().toBuffer(),
      documentTransitions[3].getId().toBuffer(),
      documentTransitions[4].getId().toBuffer(),
      documentTransitions[4].getId().toBuffer(),
    ]);

    expect(errors.map((e) => e.getDuplicatingProperties())).to.have.deep.members([
      indicesDefinition[0].properties.map((i) => Object.keys(i)[0]),
      indicesDefinition[1].properties.map((i) => Object.keys(i)[0]),
      indicesDefinition[0].properties.map((i) => Object.keys(i)[0]),
      indicesDefinition[1].properties.map((i) => Object.keys(i)[0]),
    ]);
  });

  it('should return valid result if Document has undefined field from index', async () => {
    const indexedDocument = documents[7];
    const indexedDocumentTransitions = getDocumentTransitionsFixture({
      create: [indexedDocument],
    });

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId().toBuffer(),
        indexedDocument.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
            ['firstName', '==', indexedDocument.get('firstName')],
          ],
        },
      )
      .resolves([indexedDocument]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId(),
        indexedDocument.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId],
          ],
        },
      )
      .resolves([indexedDocument]);

    const result = await validateDocumentsUniquenessByIndices(
      ownerId,
      indexedDocumentTransitions,
      dataContract,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if Document being created and has createdAt and updatedAt indices', async () => {
    const [, , , , , , uniqueDatesDocument] = documents;

    const uniqueDatesDocumentTransitions = getDocumentTransitionsFixture({
      create: [uniqueDatesDocument],
    });
    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId().toBuffer(),
        uniqueDatesDocument.getType(),
        {
          where: [
            ['$createdAt', '==', uniqueDatesDocument.getCreatedAt().getTime()],
            ['$updatedAt', '==', uniqueDatesDocument.getUpdatedAt().getTime()],
          ],
        },
      )
      .resolves([uniqueDatesDocument]);

    const result = await validateDocumentsUniquenessByIndices(
      ownerId,
      uniqueDatesDocumentTransitions,
      dataContract,
      executionContext,
    );

    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result on dry run', async () => {
    const [niceDocument] = documents;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    executionContext.enableDryRun();

    const result = await validateDocumentsUniquenessByIndices(
      ownerId,
      noIndexDocumentTransitions,
      dataContract,
      executionContext,
    );
    executionContext.disableDryRun();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMock.fetchDocuments).to.have.not.been.called();
  });
});
