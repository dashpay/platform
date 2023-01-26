const verifyDocumentsUniquenessByIndicesFactory = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/validation/state/validateDocumentsUniquenessByIndicesFactory');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const { expectValidationError: expectValidationErrorJs } = require('@dashevo/dpp/lib/test/expect/expectError');
const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const ValidationResultJs = require('@dashevo/dpp/lib/validation/ValidationResult');

const DuplicateUniqueIndexErrorJs = require('@dashevo/dpp/lib/errors/consensus/state/document/DuplicateUniqueIndexError');
const StateTransitionExecutionContextJs = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const sinon = require('sinon');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError')
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



describe('validateDocumentsUniquenessByIndices', () => {
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let validateDocumentsUniquenessByIndicesJs;
  let documentsJs;
  let documentTransitionsJs;
  let documentTransitions;
  let dataContractJs;
  let dataContract;
  let ownerIdJs;
  let ownerId;
  let executionContextJs;
  let executionContext;

  beforeEach(async function beforeEach() {
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
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    documentsJs = getDocumentsFixture(dataContractJs);
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: documentsJs,
    });

    documentTransitions = documentTransitionsJs.map((transition) =>
      DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(transition.toObject(), dataContract.clone()))
    );

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDocuments.returns([]);

    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMockJs.fetchDocuments.resolves([]);

    executionContext = new StateTransitionExecutionContext();
    executionContextJs = new StateTransitionExecutionContextJs();

    validateDocumentsUniquenessByIndicesJs = verifyDocumentsUniquenessByIndicesFactory(
      stateRepositoryMockJs,
    );
  });

  it('should return valid result if Documents have no unique indices', async () => {
    const [niceDocument] = documentsJs;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    const result = await validateDocumentsUniquenessByIndicesJs(
      ownerIdJs,
      noIndexDocumentTransitions,
      dataContractJs,
      executionContextJs,
    );

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMockJs.fetchDocuments).to.have.not.been.called();
  });


  it('should return valid result if Documents have no unique indices - Rust', async () => {
    const [niceDocument] = documentsJs;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    const documentTransition = DocumentTransition.fromTransitionCreate(
      new DocumentCreateTransition(noIndexDocumentTransitions[0].toObject(), dataContract)
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

  it('should return valid result if Document has unique indices and there are no duplicates', async () => {
    const [, , , william] = documentsJs;

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId().toBuffer(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([william]);

    stateRepositoryMockJs.fetchDocuments
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
      .resolves([william]);

    const result = await validateDocumentsUniquenessByIndicesJs(
      ownerIdJs,
      documentTransitionsJs,
      dataContractJs,
      executionContextJs,
    );

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if Document has unique indices and there are no duplicates - Rust', async () => {
    const [, , , william] = documentsJs;
    const williamDocument = new Document(william.toObject(), dataContract);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        dataContract.getId().toBuffer(),
        williamDocument.getType(),
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

  it('should return invalid result if Document has unique indices and there are duplicates', async () => {
    const [, , , william, leon] = documentsJs;

    const indicesDefinition = dataContractJs.getDocumentSchema(william.getType()).indices;

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .resolves([leon]);

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId(),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .resolves([leon]);

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId(),
        leon.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['firstName', '==', leon.get('firstName')],
          ],
        },
      )
      .resolves([william]);

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId(),
        leon.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['lastName', '==', leon.get('lastName')],
          ],
        },
      )
      .resolves([william]);

    const result = await validateDocumentsUniquenessByIndicesJs(
      ownerIdJs,
      documentTransitionsJs,
      dataContractJs,
      executionContextJs,
    );

    expectValidationErrorJs(result, DuplicateUniqueIndexErrorJs, 4);

    const errors = result.getErrors();

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4009);

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

  it('should return invalid result if Document has unique indices and there are duplicates - Rust', async () => {
    let [, , , william, leon] = documentsJs;

    william = new Document(william.toObject(), dataContract.clone());
    leon = new Document(leon.toObject(), dataContract.clone());

    const indicesDefinition = dataContractJs.getDocumentSchema(william.getType()).indices;

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
            ['firstName', '==', william.get('firstName')],
          ],
        },
      )
      .returns([leon]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        william.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
            ['lastName', '==', william.get('lastName')],
          ],
        },
      )
      .returns([leon]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        leon.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
            ['firstName', '==', leon.get('firstName')],
          ],
        },
      )
      .returns([william]);

    stateRepositoryMock.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        leon.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
            ['lastName', '==', leon.get('lastName')],
          ],
        },
      )
      .returns([william]);

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
    expect(error.getCode()).to.equal(4009);

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

  it('should return valid result if Document has undefined field from index', async () => {
    const indexedDocument = documentsJs[7];
    const indexedDocumentTransitions = getDocumentTransitionsFixture({
      create: [indexedDocument],
    });

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId().toBuffer(),
        indexedDocument.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
            ['firstName', '==', indexedDocument.get('firstName')],
          ],
        },
      )
      .resolves([indexedDocument]);

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId(),
        indexedDocument.getType(),
        {
          where: [
            ['$ownerId', '==', ownerIdJs],
          ],
        },
      )
      .resolves([indexedDocument]);

    const result = await validateDocumentsUniquenessByIndicesJs(
      ownerIdJs,
      indexedDocumentTransitions,
      dataContractJs,
      executionContextJs,
    );

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if Document has undefined field from index - Rust', async () => {
    const indexedDocumentJs = documentsJs[7];
    const indexedDocument = new Document(indexedDocumentJs.toObject(), dataContract.clone());
    const indexedDocumentTransitions = getDocumentTransitionsFixture({
      create: [indexedDocumentJs],
    }).map((t) =>
      DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(t.toObject(), dataContract.clone())

      )
    );

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        indexedDocument.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
            ['firstName', '==', indexedDocument.get('firstName')],
          ],
        },
      )
      .returns([indexedDocument]);

    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        sinon.match.instanceOf(Identifier),
        indexedDocument.getType(),
        {
          where: [
            ['$ownerId', '==', ownerId.toJSON()],
          ],
        },
      )
      .returns([indexedDocument]);

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

  it('should return valid result if Document being created and has createdAt and updatedAt indices', async () => {
    const [, , , , , , uniqueDatesDocument] = documentsJs;

    const uniqueDatesDocumentTransitions = getDocumentTransitionsFixture({
      create: [uniqueDatesDocument],
    });
    stateRepositoryMockJs.fetchDocuments
      .withArgs(
        dataContractJs.getId().toBuffer(),
        uniqueDatesDocument.getType(),
        {
          where: [
            ['$createdAt', '==', uniqueDatesDocument.getCreatedAt().getTime()],
            ['$updatedAt', '==', uniqueDatesDocument.getUpdatedAt().getTime()],
          ],
        },
      )
      .resolves([uniqueDatesDocument]);

    const result = await validateDocumentsUniquenessByIndicesJs(
      ownerIdJs,
      uniqueDatesDocumentTransitions,
      dataContractJs,
      executionContextJs,
    );

    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if Document being created and has createdAt and updatedAt indices - Rust', async () => {
    const [, , , , , , uniqueDatesDocumentJs] = documentsJs;
    const uniqueDatesDocument = new Document(uniqueDatesDocumentJs.toObject(), dataContract.clone());
    const uniqueDatesDocumentTransitions = getDocumentTransitionsFixture({
      create: [uniqueDatesDocumentJs],
    }).map((t) =>
      DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(t.toObject(), dataContract.clone())
      )
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
      .returns([uniqueDatesDocument]);

    const result = await validateDocumentsUniquenessByIndices(
      stateRepositoryMock,
      ownerId,
      uniqueDatesDocumentTransitions,
      dataContract,
      executionContext,
    );

    expect(result.isValid()).to.be.true();
  });

  it('should return invalid result on dry run', async () => {
    const [niceDocument] = documentsJs;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    executionContextJs.enableDryRun();

    const result = await validateDocumentsUniquenessByIndicesJs(
      ownerIdJs,
      noIndexDocumentTransitions,
      dataContractJs,
      executionContextJs,
    );
    executionContextJs.disableDryRun();

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();
    expect(stateRepositoryMockJs.fetchDocuments).to.have.not.been.called();
  });

  it('should return invalid result on dry run - Rust', async () => {
    const [niceDocument] = documentsJs;
    const noIndexDocumentTransitions = getDocumentTransitionsFixture({
      create: [niceDocument],
    });

    executionContext.enableDryRun();
    const documentTransition = DocumentTransition.fromTransitionCreate(
      new DocumentCreateTransition(noIndexDocumentTransitions[0].toObject(), dataContract)
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
