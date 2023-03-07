const DocumentJs = require('@dashevo/dpp/lib/document/Document');
const DocumentsBatchTransitionJs = require(
  '@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition',
);

const applyDocumentsBatchTransitionFactory = require(
  '@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/applyDocumentsBatchTransitionFactory',
);

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture',
);

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const StateTransitionExecutionContextJs = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const { default: loadWasmDpp } = require('../../../../../dist');

let Document;
let DocumentsBatchTransition;
let DataContract;
let StateTransitionExecutionContext;
let applyDocumentsBatchTransition;
let DocumentNotProvidedError;

describe('applyDocumentsBatchTransitionFactory', () => {
  let documentsJs;
  let dataContractJs;
  let dataContract;
  let documentTransitionsJs;
  let ownerId;
  let replaceDocumentJs;
  let replaceDocument;
  let stateTransitionJs;
  let stateTransition;
  let documentsFixtureJs;
  let documentsFixture;
  let applyDocumentsBatchTransitionJs;
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let fetchDocumentsMock;
  let executionContextJs;
  let executionContext;
  let blockTimeMs;

  beforeEach(async function beforeEach() {
    ({
      DataContract,
      Document,
      DocumentsBatchTransition,
      StateTransitionExecutionContext,
      applyDocumentsBatchTransition,
      // Errors:
      DocumentNotProvidedError,
    } = await loadWasmDpp());

    dataContractJs = getDataContractFixture();
    dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    documentsFixtureJs = getDocumentsFixture(dataContractJs);
    documentsFixture = documentsFixtureJs.map((d) => {
      const doc = new Document(d.toObject(), dataContract);
      doc.setEntropy(d.entropy);
      return doc;
    });

    ownerId = getDocumentsFixture.ownerId;

    replaceDocumentJs = new DocumentJs({
      ...documentsFixtureJs[1].toObject(),
      lastName: 'NotSoShiny',
    }, dataContractJs);

    replaceDocument = new Document({
      ...documentsFixture[1].toObject(),
      lastName: 'NotSoShiny',
    }, dataContract);

    blockTimeMs = Date.now();
    documentsJs = [replaceDocumentJs, documentsFixtureJs[2]];
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [documentsFixtureJs[0]],
      replace: [documentsJs[0]],
      delete: [documentsJs[1]],
    });

    stateTransitionJs = new DocumentsBatchTransitionJs({
      protocolVersion: protocolVersion.latestVersion,
      ownerId,
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContractJs]);

    stateTransition = new DocumentsBatchTransition({
      protocolVersion: protocolVersion.latestVersion,
      ownerId,
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract]);

    executionContextJs = new StateTransitionExecutionContextJs();
    executionContext = new StateTransitionExecutionContext();

    stateTransitionJs.setExecutionContext(executionContextJs);
    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMockJs.fetchDataContract.resolves(dataContractJs);
    stateRepositoryMockJs.fetchLatestPlatformBlockTime.resolves(blockTimeMs);

    fetchDocumentsMock = this.sinonSandbox.stub();
    fetchDocumentsMock.resolves([
      replaceDocumentJs,
    ]);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves(blockTimeMs);
    stateRepositoryMock.updateDocument.resolves(null);
    stateRepositoryMock.removeDocument.resolves(null);
    stateRepositoryMock.createDocument.resolves(null);
    stateRepositoryMock.fetchDocuments.resolves([replaceDocument]);

    applyDocumentsBatchTransitionJs = applyDocumentsBatchTransitionFactory(
      stateRepositoryMockJs,
      fetchDocumentsMock,
    );
  });

  it('should call `store`, `replace` and `remove` functions for specific type of transitions', async () => {
    await applyDocumentsBatchTransitionJs(stateTransitionJs);

    const replaceDocumentTransition = documentTransitionsJs[1];

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
      [replaceDocumentTransition],
      executionContextJs,
    );

    expect(stateRepositoryMockJs.createDocument).to.have.been.calledOnce();
    expect(stateRepositoryMockJs.updateDocument).to.have.been.calledOnce();

    const callsArgs = [
      ...stateRepositoryMockJs.createDocument.getCall(0).args,
      ...stateRepositoryMockJs.updateDocument.getCall(0).args,
    ];

    expect(callsArgs).to.have.deep.members([
      documentsFixtureJs[0],
      documentsJs[0],
      executionContextJs,
      executionContextJs,
    ]);

    expect(stateRepositoryMockJs.removeDocument).to.have.been.calledOnceWithExactly(
      documentTransitionsJs[2].getDataContract(),
      documentTransitionsJs[2].getType(),
      documentTransitionsJs[2].getId(),
      executionContextJs,
    );
  });

  it('should call `store`, `replace` and `remove` functions for specific type of transitions - Rust', async () => {
    await applyDocumentsBatchTransition(stateRepositoryMock, stateTransition);
    expect(stateRepositoryMock.createDocument).to.have.been.calledOnce();

    const [fetchContractId, fetchDocumentType] = stateRepositoryMock.fetchDocuments.getCall(0).args;
    expect(fetchContractId.toBuffer()).to.deep.equal(documentTransitionsJs[1].getDataContractId());
    expect(fetchDocumentType).to.equal(documentTransitionsJs[1].getType());

    expect(stateRepositoryMock.updateDocument).to.have.been.calledOnce();
    expect(stateRepositoryMock.fetchDocuments).to.have.been.calledOnce();

    const [createDocument] = stateRepositoryMock.createDocument.getCall(0).args;
    const [updateDocument] = stateRepositoryMock.updateDocument.getCall(0).args;
    const [
      deleteDataContract,
      deleteDocumentType,
      deleteDocumentId,
    ] = stateRepositoryMock.removeDocument.getCall(0).args;

    expect(createDocument.toObject()).to.deep.equal(documentsFixtureJs[0].toObject());
    const expectDocument = documentsJs[0].toJSON();

    // ! Why we need to replace. Apparently, `applyDocumentsTransition` somehow
    // ! modifies `documentsJs` and increments revision and $updatedAt. I have
    // ! no clue how it's possible.
    expectDocument.$updatedAt = documentTransitionsJs[1].toJSON().$updatedAt;
    expectDocument.$revision = documentTransitionsJs[1].toJSON().$revision;
    expect(updateDocument.toJSON()).to.deep.equal(expectDocument);

    expect(deleteDataContract.toObject()).to.deep.equal(dataContract.toObject());
    expect(deleteDocumentType).to.deep.equal(documentTransitionsJs[2].getType());
    expect(deleteDocumentId.toBuffer()).to.deep.equal(documentTransitionsJs[2].getId());
  });

  it('should throw an error if document was not provided for a replacement - Rust', async () => {
    stateRepositoryMock.fetchDocuments.resolves([]);

    const replaceDocumentTransition = documentTransitionsJs[1];

    try {
      await applyDocumentsBatchTransition(stateRepositoryMock, stateTransition);
      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(DocumentNotProvidedError);
      expect(e.getDocumentTransition().toObject())
        .to.deep.equal(replaceDocumentTransition.toObject());
    }
  });

  it('should call `replace` functions on dry run - Rust', async function test() {
    this.timeout(10000);
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves(blockTimeMs);
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: [],
      replace: [documentsJs[0]],
      delete: [],
    });

    stateTransition = new DocumentsBatchTransition({
      protocolVersion: protocolVersion.latestVersion,
      ownerId,
      transitions: documentTransitionsJs.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.getExecutionContext().enableDryRun();

    await applyDocumentsBatchTransition(stateRepositoryMock, stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    expect(stateRepositoryMock.fetchLatestPlatformBlockTime).to.have.been.calledOnceWith();

    const [documentTransition] = stateTransition.getTransitions();

    const newDocument = new Document({
      $protocolVersion: stateTransitionJs.getProtocolVersion(),
      $id: documentTransition.getId(),
      $type: documentTransition.getType(),
      $dataContractId: documentTransition.getDataContractId(),
      $ownerId: stateTransitionJs.getOwnerId(),
      $createdAt: blockTimeMs,
      ...documentTransition.getData(),
    }, documentTransition.getDataContract());

    newDocument.setRevision(documentTransition.getRevision());
    newDocument.setData(documentTransition.getData());
    newDocument.setUpdatedAt(documentTransition.getUpdatedAt());

    expect(stateRepositoryMock.updateDocument).to.have.been.called.calledOnce();
    const [updateDocument] = stateRepositoryMock.updateDocument.getCall(0).args;

    expect(updateDocument.toObject()).to.be.deep.equal(newDocument.toObject());
  });
});
