const getDocumentsFixture = require('../../../../lib/test/fixtures/getDocumentsFixture');
const getContractFixture = require('../../../../lib/test/fixtures/getContractFixture');

const fetchDocumentsByDocumentsFactory = require('../../../../lib/stPacket/verification/fetchDocumentsByDocumentsFactory');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');

describe('fetchDocumentsByDocuments', () => {
  let fetchDocumentsByDocuments;
  let dataProviderMock;
  let documents;
  let contract;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    fetchDocumentsByDocuments = fetchDocumentsByDocumentsFactory(dataProviderMock);

    documents = getDocumentsFixture();
    contract = getContractFixture();
  });

  it('should fetch specified Documents using DataProvider', async () => {
    dataProviderMock.fetchDocuments.withArgs(
      contract.getId(),
      documents[0].getType(),
    ).resolves([documents[0]]);

    dataProviderMock.fetchDocuments.withArgs(
      contract.getId(),
      documents[1].getType(),
    ).resolves([documents[1], documents[2]]);

    dataProviderMock.fetchDocuments.withArgs(
      contract.getId(),
      documents[3].getType(),
    ).resolves([documents[3], documents[4]]);

    const fetchedDocuments = await fetchDocumentsByDocuments(contract.getId(), documents);

    expect(dataProviderMock.fetchDocuments).to.have.been.calledThrice();

    const callArgsOne = [
      contract.getId(),
      documents[0].getType(),
      {
        where: [
          ['$id', 'in', [documents[0].getId()]],
        ],
      },
    ];

    const callArgsTwo = [
      contract.getId(),
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
      contract.getId(),
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
