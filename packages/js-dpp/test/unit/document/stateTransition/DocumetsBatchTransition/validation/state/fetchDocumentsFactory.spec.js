const getDocumentsFixture = require('../../../../../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../../../../../lib/test/fixtures/getDocumentTransitionsFixture');

const fetchDocumentsFactory = require('../../../../../../../lib/document/stateTransition/DocumentsBatchTransition/validation/state/fetchDocumentsFactory');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');

const generateRandomIdentifier = require('../../../../../../../lib/test/utils/generateRandomIdentifier');
const StateTransitionExecutionContext = require('../../../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('fetchDocumentsFactory', () => {
  let fetchDocuments;
  let stateRepositoryMock;
  let documentTransitions;
  let documents;
  let executionContext;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    fetchDocuments = fetchDocumentsFactory(stateRepositoryMock);

    executionContext = new StateTransitionExecutionContext();

    documents = getDocumentsFixture().slice(0, 5);

    documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });
  });

  it('should fetch specified Documents using StateRepository', async () => {
    const firstDocumentDataContractId = generateRandomIdentifier().toBuffer();

    documentTransitions[0].dataContractId = firstDocumentDataContractId;
    documents[0].dataContractId = firstDocumentDataContractId;

    stateRepositoryMock.fetchDocuments.withArgs(
      documentTransitions[0].getDataContractId(),
      documentTransitions[0].getType(),
    ).resolves([documents[0]]);

    stateRepositoryMock.fetchDocuments.withArgs(
      documentTransitions[1].getDataContractId(),
      documentTransitions[1].getType(),
    ).resolves([documents[1], documents[2]]);

    stateRepositoryMock.fetchDocuments.withArgs(
      documentTransitions[3].getDataContractId(),
      documentTransitions[3].getType(),
    ).resolves([documents[3], documents[4]]);

    const fetchedDocuments = await fetchDocuments(documentTransitions, executionContext);

    expect(stateRepositoryMock.fetchDocuments).to.have.been.calledThrice();

    const callArgsOne = [
      documents[0].getDataContractId(),
      documents[0].getType(),
      {
        where: [
          ['$id', 'in', [documents[0].getId()]],
        ],
        orderBy: [
          [
            '$id',
            'asc',
          ],
        ],
      },
      executionContext,
    ];

    const callArgsTwo = [
      documents[1].getDataContractId(),
      documents[1].getType(),
      {
        where: [
          ['$id', 'in', [
            documents[1].getId(),
            documents[2].getId(),
          ]],
        ],
        orderBy: [
          [
            '$id',
            'asc',
          ],
        ],
      },
      executionContext,
    ];

    const callArgsThree = [
      documents[3].getDataContractId(),
      documents[3].getType(),
      {
        where: [
          ['$id', 'in', [
            documents[3].getId(),
            documents[4].getId(),
          ]],
        ],
        orderBy: [
          [
            '$id',
            'asc',
          ],
        ],
      },
      executionContext,
    ];

    const callsArgs = [];
    for (let i = 0; i < stateRepositoryMock.fetchDocuments.callCount; i++) {
      const call = stateRepositoryMock.fetchDocuments.getCall(i);
      callsArgs.push(call.args);
    }

    expect(callsArgs).to.have.deep.members([
      callArgsOne,
      callArgsTwo,
      callArgsThree,
    ]);

    expect(fetchedDocuments).to.deep.equal(documents);
  });
});
