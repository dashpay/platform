const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const generateRandomIdentifierJs = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');

const sinon = require('sinon');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');
const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const { default: loadWasmDpp } = require('../../../../../../../dist');

let Identifier;
let DataContract;
let fetchExtendedDocuments;
let DocumentTransition;
let DocumentCreateTransition;
let StateTransitionExecutionContext;
let ExtendedDocument;

describe.skip('fetchDocumentsFactory', () => {
  let stateRepositoryMock;
  let documentTransitionsJs;
  let documentTransitions;
  let documentsJs;
  let documents;
  let executionContext;

  beforeEach(async function beforeEach() {
    ({
      Identifier,
      DataContract,
      DocumentTransition,
      DocumentCreateTransition,
      StateTransitionExecutionContext,
      fetchExtendedDocuments,
      ExtendedDocument,
    } = await loadWasmDpp());

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    executionContext = new StateTransitionExecutionContext();

    documentsJs = getDocumentsFixture().slice(0, 5);
    const dataContractBuffer = documentsJs[0].dataContract.toBuffer();
    const dataContract = DataContract.fromBuffer(dataContractBuffer);

    documents = documentsJs.map((document) => {
      document.toObject();

      return new ExtendedDocument(
        document.toObject(), dataContract.clone(), // document.getType(),
      );
    });
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

    stateRepositoryMock.fetchExtendedDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitions[0].getType(),
    ).resolves([documents[0]]);

    stateRepositoryMock.fetchExtendedDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitions[1].getType(),
    ).resolves([documents[1], documents[2]]);

    stateRepositoryMock.fetchExtendedDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitionsJs[3].getType(),
    ).resolves([documents[3], documents[4]]);

    await fetchExtendedDocuments(
      stateRepositoryMock,
      documentTransitions,
      executionContext,
    );

    expect(stateRepositoryMock.fetchExtendedDocuments).to.have.been.calledThrice();
  });
});
