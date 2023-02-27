const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const fetchDocumentsFactory = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/validation/state/fetchDocumentsFactory');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const generateRandomIdentifierJs = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const StateTransitionExecutionContextJs = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const sinon = require('sinon');
const { default: loadWasmDpp } = require('../../../../../../../dist');

let Identifier;
let DataContract;
let Document;
let fetchDocuments;
let DocumentTransition;
let DocumentCreateTransition;
let StateTransitionExecutionContext;

describe('fetchDocumentsFactory', () => {
  let fetchDocumentsJs;
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let documentTransitionsJs;
  let documentTransitions;
  let documentsJs;
  let documents;
  let executionContextJs;
  let executionContext;

  beforeEach(async function beforeEach() {
    ({
      Identifier,
      Document,
      DataContract,
      DocumentTransition,
      DocumentCreateTransition,
      StateTransitionExecutionContext,
      fetchDocuments,
    } = await loadWasmDpp());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);

    fetchDocumentsJs = fetchDocumentsFactory(stateRepositoryMockJs);

    executionContext = new StateTransitionExecutionContext();
    executionContextJs = new StateTransitionExecutionContextJs();

    documentsJs = getDocumentsFixture().slice(0, 5);
    const dataContractBuffer = documentsJs[0].dataContract.toBuffer();
    const dataContract = DataContract.fromBuffer(dataContractBuffer);

    documents = documentsJs.map((document) => new Document(
      document.toObject(), dataContract.clone(),
    ));
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: documentsJs,
    });
    documentTransitions = documentTransitionsJs.map(
      (transition) => DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(
          transition.toObject(), dataContract.clone(),
        ),
      ),
    );
  });

  it('should fetch specified Documents using StateRepository - Rust', async () => {
    const firstDocumentDataContractId = generateRandomIdentifierJs().toBuffer();

    documentTransitions[0].setDataContractId(firstDocumentDataContractId);
    documents[0].setDataContractId(firstDocumentDataContractId);

    stateRepositoryMock.fetchDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitions[0].getType(),
    ).resolves([documents[0]]);

    stateRepositoryMock.fetchDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitions[1].getType(),
    ).resolves([documents[1], documents[2]]);

    stateRepositoryMock.fetchDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitionsJs[3].getType(),
    ).resolves([documents[3], documents[4]]);

    await fetchDocuments(
      stateRepositoryMock,
      documentTransitions,
      executionContext,
    );

    expect(stateRepositoryMock.fetchDocuments).to.have.been.calledThrice();
  });
});
