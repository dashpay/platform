const Document = require('@dashevo/dpp/lib/document/Document');
const DocumentsBatchTransition = require(
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
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');
const DocumentNotProvidedError = require('@dashevo/dpp/lib/document/errors/DocumentNotProvidedError');

describe('applyDocumentsBatchTransitionFactory', () => {
  let documents;
  let dataContract;
  let documentTransitions;
  let ownerId;
  let replaceDocument;
  let stateTransition;
  let documentsFixture;
  let applyDocumentsBatchTransition;
  let stateRepositoryMock;
  let fetchDocumentsMock;
  let executionContext;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    documentsFixture = getDocumentsFixture(dataContract);

    ownerId = getDocumentsFixture.ownerId;

    replaceDocument = new Document({
      ...documentsFixture[1].toObject(),
      lastName: 'NotSoShiny',
    }, dataContract);

    documents = [replaceDocument, documentsFixture[2]];

    documentTransitions = getDocumentTransitionsFixture({
      create: [documentsFixture[0]],
      replace: [documents[0]],
      delete: [documents[1]],
    });

    stateTransition = new DocumentsBatchTransition({
      protocolVersion: protocolVersion.latestVersion,
      ownerId,
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves({
      seconds: 86400,
    });

    fetchDocumentsMock = this.sinonSandbox.stub();
    fetchDocumentsMock.resolves([
      replaceDocument,
    ]);

    applyDocumentsBatchTransition = applyDocumentsBatchTransitionFactory(
        stateRepositoryMock,
        fetchDocumentsMock,
    );
  });

  it('should call `store`, `replace` and `remove` functions for specific type of transitions', async () => {
    await applyDocumentsBatchTransition(stateTransition);

    const replaceDocumentTransition = documentTransitions[1];

    expect(fetchDocumentsMock).to.have.been.calledOnceWithExactly(
        [replaceDocumentTransition],
        executionContext,
    );

    expect(stateRepositoryMock.createDocument).to.have.been.calledOnce();
    expect(stateRepositoryMock.updateDocument).to.have.been.calledOnce();

    const callsArgs = [
      ...stateRepositoryMock.createDocument.getCall(0).args,
      ...stateRepositoryMock.updateDocument.getCall(0).args,
    ];

    expect(callsArgs).to.have.deep.members([
      documentsFixture[0],
      documents[0],
      executionContext,
      executionContext,
    ]);

    expect(stateRepositoryMock.removeDocument).to.have.been.calledOnceWithExactly(
        documentTransitions[2].getDataContract(),
        documentTransitions[2].getType(),
        documentTransitions[2].getId(),
        executionContext,
    );
  });

  it('should throw an error if document was not provided for a replacement', async () => {
    fetchDocumentsMock.resolves([]);

    const replaceDocumentTransition = documentTransitions[1];

    try {
      await applyDocumentsBatchTransition(stateTransition);
      expect.fail('Error was not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(DocumentNotProvidedError);
      expect(e.getDocumentTransition()).to.deep.equal(replaceDocumentTransition);
    }
  });

  it('should call `replace` functions on dry run', async () => {
    documentTransitions = getDocumentTransitionsFixture({
      create: [],
      replace: [documents[0]],
      delete: [],
    });

    stateTransition = new DocumentsBatchTransition({
      protocolVersion: protocolVersion.latestVersion,
      ownerId,
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    stateTransition.getExecutionContext().enableDryRun();

    await applyDocumentsBatchTransition(stateTransition);

    stateTransition.getExecutionContext().disableDryRun();

    expect(stateRepositoryMock.fetchLatestPlatformBlockTime).to.have.been.calledOnceWith();

    const [documentTransition] = stateTransition.getTransitions();
    const newDocument = new Document({
      $protocolVersion: stateTransition.getProtocolVersion(),
      $id: documentTransition.getId(),
      $type: documentTransition.getType(),
      $dataContractId: documentTransition.getDataContractId(),
      $ownerId: stateTransition.getOwnerId(),
      $createdAt: 86400 * 1000,
      ...documentTransition.getData(),
    }, documentTransition.getDataContract());

    newDocument.setRevision(documentTransition.getRevision());
    newDocument.setData(documentTransition.getData());
    newDocument.setUpdatedAt(documentTransition.getUpdatedAt());

    expect(stateRepositoryMock.updateDocument).to.have.been.calledOnceWithExactly(
        newDocument,
        executionContext,
    );
  });
});
