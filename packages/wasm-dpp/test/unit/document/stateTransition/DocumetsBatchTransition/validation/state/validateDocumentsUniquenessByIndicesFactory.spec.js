const sinon = require('sinon');
const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');
const { default: loadWasmDpp } = require('../../../../../../../dist');

let DataContract;
let Document;
let ValidationResult;
let DuplicateUniqueIndexError;
let validateDocumentsUniquenessByIndices;
let DocumentCreateTransition;
let DocumentTransition;
let Identifier;
let StateTransitionExecutionContext;

describe.skip('validateDocumentsUniquenessByIndices', () => {
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let documentsJs;
  let documentTransitionsJs;
  let documentTransitions;
  let dataContractJs;
  let dataContract;
  let ownerIdJs;
  let ownerId;
  let executionContext;

  beforeEach(async () => {
    ({
      Document,
      DataContract,
      Identifier,
      DocumentCreateTransition,
      DocumentTransition,
      ValidationResult,
      StateTransitionExecutionContext,
      validateDocumentsUniquenessByIndices,

      DuplicateUniqueIndexError,
    } = await loadWasmDpp());

    ({ ownerId: ownerIdJs } = getDocumentsFixture);
    ownerId = Identifier.from(ownerIdJs.toBuffer());

    dataContractJs = getContractFixture();
    dataContract = new DataContract(dataContractJs.toObject());

    documentsJs = getDocumentsFixture(dataContractJs);
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: documentsJs,
    });

    documentTransitions = documentTransitionsJs.map(
      (transition) => DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(transition.toObject(), dataContract.clone()),
      ),
    );

    executionContext = new StateTransitionExecutionContext();
  });

  it('should return valid result if Documents have no unique indices - Rust', async () => {
    const [niceDocument] = documentsJs;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    const documentTransition = DocumentTransition.fromTransitionCreate(
      new DocumentCreateTransition(noIndexDocumentTransitions[0].toObject(), dataContract),
    );

    const result = await validateDocumentsUniquenessByIndices(
      stateRepositoryMock,
      ownerId,
      [documentTransition],
      dataContract,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMock.fetchDocuments).to.have.not.been.called();
  });

  it('should return valid result if Document has unique indices and there are no duplicates - Rust', async () => {
    const [, , , william] = documentsJs;
    const williamDocument = new Document(william.toObject(), dataContract, william.getType());

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId().toBuffer(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([williamDocument]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContractJs.getId().toBuffer(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .resolves([williamDocument]);

    const result = await validateDocumentsUniquenessByIndices(
      stateRepositoryMock,
      ownerId,
      documentTransitions,
      dataContract,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result if Document has unique indices and there are duplicates - Rust', async () => {
    let [, , , william, leon] = documentsJs;

    const williamType = william.getType();
    const leonType = leon.getType();

    william = new Document(william.toObject(), dataContract.clone(), williamType);
    leon = new Document(leon.toObject(), dataContract.clone(), leonType);

    const indicesDefinition = dataContractJs.getDocumentSchema(williamType).indices;

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        williamType,
        {
          where: [
            ['$ownerId', '==', Array.from(ownerId)],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([leon]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        williamType,
        {
          where: [
            ['$ownerId', '==', Array.from(ownerId)],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .resolves([leon]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        leonType,
        {
          where: [
            ['$ownerId', '==', Array.from(ownerId)],
            ['firstName', '==', leon.get('firstName')],
          ],
        },
      )
      .resolves([william]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        leonType,
        {
          where: [
            ['$ownerId', '==', Array.from(ownerId)],
            ['lastName', '==', leon.get('lastName')],
          ],
        },
      )
      .resolves([william]);

    const result = await validateDocumentsUniquenessByIndices(
      stateRepositoryMock,
      ownerId,
      documentTransitions,
      dataContract,
      executionContext,
    );

    await expectValidationError(result, DuplicateUniqueIndexError, 4);
    const errors = result.getErrors();
    const [error] = result.getErrors();
    expect(error.getCode()).to.equal(40105);

    expect(errors.map((e) => e.getDocumentId())).to.have.deep.members([
      documentTransitionsJs[3].getId().toBuffer(),
      documentTransitionsJs[3].getId().toBuffer(),
      documentTransitionsJs[4].getId().toBuffer(),
      documentTransitionsJs[4].getId().toBuffer(),
    ]);

    expect(errors.map((e) => e.getDuplicatingProperties())).to.have.deep.members([
      indicesDefinition[0].properties.map((i) => Object.keys(i)[0]),
      indicesDefinition[1].properties.map((i) => Object.keys(i)[0]),
      indicesDefinition[0].properties.map((i) => Object.keys(i)[0]),
      indicesDefinition[1].properties.map((i) => Object.keys(i)[0]),
    ]);
  });

  it('should return valid result if Document has undefined field from index - Rust', async () => {
    const indexedDocumentJs = documentsJs[7];
    const indexedDocument = new Document(
      indexedDocumentJs.toObject(),
      dataContract.clone(),
      indexedDocumentJs.getType(),
    );
    const indexedDocumentTransitions = getDocumentTransitionsFixture({
      create: [indexedDocumentJs],
    }).map(
      (t) => DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(t.toObject(), dataContract.clone()),
      ),
    );

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        indexedDocumentJs.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
            ['firstName', '==', indexedDocument.get('firstName')],
          ],
        },
      )
      .resolves([indexedDocument]);

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        indexedDocumentJs.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
          ],
        },
      )
      .resolves([indexedDocument]);

    const result = await validateDocumentsUniquenessByIndices(
      stateRepositoryMock,
      ownerId,
      indexedDocumentTransitions,
      dataContract,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if Document being created and has createdAt and updatedAt indices - Rust', async () => {
    const [, , , , , , uniqueDatesDocumentJs] = documentsJs;
    const uniqueDatesDocument = new Document(
      uniqueDatesDocumentJs.toObject(),
      dataContract.clone(),
      uniqueDatesDocumentJs.getType(),
    );
    const uniqueDatesDocumentTransitions = getDocumentTransitionsFixture({
      create: [uniqueDatesDocumentJs],
    }).map(
      (t) => DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(t.toObject(), dataContract.clone()),
      ),
    );

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        uniqueDatesDocumentJs.getType(),
        {
          where: [
            ['$createdAt', '==', uniqueDatesDocument.getCreatedAt()],
            ['$updatedAt', '==', uniqueDatesDocument.getUpdatedAt()],
          ],
        },
      )
      .resolves([uniqueDatesDocument]);

    const result = await validateDocumentsUniquenessByIndices(
      stateRepositoryMock,
      ownerId,
      uniqueDatesDocumentTransitions,
      dataContract,
      executionContext,
    );

    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result on dry run - Rust', async () => {
    const [niceDocument] = documentsJs;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    executionContext.enableDryRun();
    const documentTransition = DocumentTransition.fromTransitionCreate(
      new DocumentCreateTransition(noIndexDocumentTransitions[0].toObject(), dataContract),
    );

    const result = await validateDocumentsUniquenessByIndices(
      stateRepositoryMock,
      ownerId,
      [documentTransition],
      dataContract,
      executionContext,
    );

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMock.fetchDocuments).to.have.not.been.called();
  });
});
