const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const fetchDocumentsFactory = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/validation/state/fetchDocumentsFactory');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const generateRandomIdentifierJs = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const StateTransitionExecutionContextJs = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const sinon = require('sinon');
const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError')
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

    documents = documentsJs.map((document) => new Document(document.toObject(), dataContract.clone()));
    documentTransitionsJs = getDocumentTransitionsFixture({
      create: documentsJs,
    });
    documentTransitions = documentTransitionsJs.map((transition) =>
      DocumentTransition.fromTransitionCreate(
        new DocumentCreateTransition(transition.toObject(), dataContract.clone())
      )
    )
  });

  it('should fetch specified Documents using StateRepository', async () => {
    const firstDocumentDataContractId = generateRandomIdentifierJs().toBuffer();

    documentTransitionsJs[0].dataContractId = firstDocumentDataContractId;
    documentsJs[0].dataContractId = firstDocumentDataContractId;

    stateRepositoryMockJs.fetchDocuments.withArgs(
      documentTransitionsJs[0].getDataContractId(),
      documentTransitionsJs[0].getType(),
    ).resolves([documentsJs[0]]);

    stateRepositoryMockJs.fetchDocuments.withArgs(
      documentTransitionsJs[1].getDataContractId(),
      documentTransitionsJs[1].getType(),
    ).resolves([documentsJs[1], documentsJs[2]]);

    stateRepositoryMockJs.fetchDocuments.withArgs(
      documentTransitionsJs[3].getDataContractId(),
      documentTransitionsJs[3].getType(),
    ).resolves([documentsJs[3], documentsJs[4]]);

    const fetchedDocuments = await fetchDocumentsJs(documentTransitionsJs, executionContextJs);

    expect(stateRepositoryMockJs.fetchDocuments).to.have.been.calledThrice();

    const callArgsOne = [
      documentsJs[0].getDataContractId(),
      documentsJs[0].getType(),
      {
        where: [
          ['$id', 'in', [documentsJs[0].getId()]],
        ],
        orderBy: [
          [
            '$id',
            'asc',
          ],
        ],
      },
      executionContextJs,
    ];

    const callArgsTwo = [
      documentsJs[1].getDataContractId(),
      documentsJs[1].getType(),
      {
        where: [
          ['$id', 'in', [
            documentsJs[1].getId(),
            documentsJs[2].getId(),
          ]],
        ],
        orderBy: [
          [
            '$id',
            'asc',
          ],
        ],
      },
      executionContextJs,
    ];

    const callArgsThree = [
      documentsJs[3].getDataContractId(),
      documentsJs[3].getType(),
      {
        where: [
          ['$id', 'in', [
            documentsJs[3].getId(),
            documentsJs[4].getId(),
          ]],
        ],
        orderBy: [
          [
            '$id',
            'asc',
          ],
        ],
      },
      executionContextJs,
    ];

    const callsArgs = [];
    for (let i = 0; i < stateRepositoryMockJs.fetchDocuments.callCount; i++) {
      const call = stateRepositoryMockJs.fetchDocuments.getCall(i);
      callsArgs.push(call.args);
    }

    expect(callsArgs).to.have.deep.members([
      callArgsOne,
      callArgsTwo,
      callArgsThree,
    ]);

    expect(fetchedDocuments).to.deep.equal(documentsJs);
  });

  it('should fetch specified Documents using StateRepository - Rust', async () => {
    const firstDocumentDataContractId = generateRandomIdentifierJs().toBuffer();

    documentTransitions[0].setDataContractId(firstDocumentDataContractId);
    documents[0].setDataContractId(firstDocumentDataContractId);

    stateRepositoryMock.fetchDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitions[0].getType(),
    ).returns([documents[0]]);

    stateRepositoryMock.fetchDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitions[1].getType(),
    ).returns([documents[1], documents[2]]);

    stateRepositoryMock.fetchDocuments.withArgs(
      sinon.match.instanceOf(Identifier),
      documentTransitionsJs[3].getType(),
    ).returns([documents[3], documents[4]]);


    const fetchedDocuments = await fetchDocuments(stateRepositoryMock, documentTransitions, executionContext);
    expect(stateRepositoryMock.fetchDocuments).to.have.been.calledThrice();
  });
});
