const getDocumentsFixture = require('../../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const fetchDocumentsByDocumentsFactory = require('../../../../../lib/document/stateTransition/validation/data/fetchDocumentsFactory');

const createDataProviderMock = require('../../../../../lib/test/mocks/createDataProviderMock');

describe('fetchDocumentsFactory', () => {
  let fetchDocuments;
  let dataProviderMock;
  let documents;
  let dataContract;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    fetchDocuments = fetchDocumentsByDocumentsFactory(dataProviderMock);

    documents = getDocumentsFixture();
    dataContract = getContractFixture();
  });

  it('should fetch specified Documents using DataProvider', async () => {
    dataProviderMock.fetchDocuments.withArgs(
      dataContract.getId(),
      documents[0].getType(),
    ).resolves([documents[0]]);

    dataProviderMock.fetchDocuments.withArgs(
      dataContract.getId(),
      documents[1].getType(),
    ).resolves([documents[1], documents[2]]);

    dataProviderMock.fetchDocuments.withArgs(
      dataContract.getId(),
      documents[3].getType(),
    ).resolves([documents[3], documents[4]]);

    const fetchedDocuments = await fetchDocuments(documents);

    expect(dataProviderMock.fetchDocuments).to.have.been.calledThrice();

    const callArgsOne = [
      dataContract.getId(),
      documents[0].getType(),
      {
        where: [
          ['$id', 'in', [documents[0].getId()]],
        ],
      },
    ];

    const callArgsTwo = [
      dataContract.getId(),
      documents[1].getType(),
      {
        where: [
          ['$id', 'in', [
            documents[1].getId(),
            documents[2].getId(),
          ]],
        ],
      },
    ];

    const callArgsThree = [
      dataContract.getId(),
      documents[3].getType(),
      {
        where: [
          ['$id', 'in', [
            documents[3].getId(),
            documents[4].getId(),
          ]],
        ],
      },
    ];

    const callsArgs = [];
    for (let i = 0; i < dataProviderMock.fetchDocuments.callCount; i++) {
      const call = dataProviderMock.fetchDocuments.getCall(i);
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
