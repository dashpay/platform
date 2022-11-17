const createStateTransitionFactory = require('@dashevo/dpp/lib/stateTransition/createStateTransitionFactory');

const DataContractCreateTransition = require('@dashevo/dpp/lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');
const DocumentsBatchTransition = require('@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/DocumentsBatchTransition');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDocumentTranstionsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentTransitionsFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const InvalidStateTransitionTypeError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionTypeError');

describe('createStateTransitionFactory', () => {
  let createStateTransition;
  let stateRepositoryMock;
  let dataContract;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);
    createStateTransition = createStateTransitionFactory(stateRepositoryMock);
  });

  it('should return DataContractCreateTransition if type is DATA_CONTRACT_CREATE', async () => {
    const stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });

    const result = await createStateTransition(stateTransition.toObject());

    expect(result).to.be.instanceOf(DataContractCreateTransition);
    expect(result.getDataContract().toObject()).to.deep.equal(dataContract.toObject());
  });

  it('should return DocumentsBatchTransition if type is DOCUMENTS', async () => {
    const documents = getDocumentsFixture(dataContract);
    const documentTransitions = getDocumentTranstionsFixture({
      create: documents,
    });

    const stateTransition = new DocumentsBatchTransition({
      ownerId: getDocumentsFixture.ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toObject()),
    }, [dataContract]);

    const result = await createStateTransition(stateTransition.toObject());

    expect(result).to.be.instanceOf(DocumentsBatchTransition);
    expect(result.getTransitions().map((t) => t.toObject())).to.have.deep.members(
      documentTransitions.map((t) => t.toObject()),
    );
  });

  it('should throw InvalidStateTransitionTypeError if type is invalid', async () => {
    const rawStateTransition = {
      type: 666,
    };

    try {
      await createStateTransition(rawStateTransition);

      expect.fail('InvalidStateTransitionTypeError is not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidStateTransitionTypeError);
      expect(e.getType()).to.equal(rawStateTransition.type);
    }
  });
});
