const createStateTransitionFactory = require('../../../lib/stateTransition/createStateTransitionFactory');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition');
const DocumentsBatchTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTranstionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const InvalidStateTransitionTypeError = require('../../../lib/errors/InvalidStateTransitionTypeError');

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
      dataContract: dataContract.toJSON(),
      entropy: dataContract.getEntropy(),
    });

    const result = await createStateTransition(stateTransition.toJSON());

    expect(result).to.be.instanceOf(DataContractCreateTransition);
    expect(result.getDataContract().toJSON()).to.deep.equal(dataContract.toJSON());
  });

  it('should return DocumentsBatchTransition if type is DOCUMENTS', async () => {
    const documents = getDocumentsFixture(dataContract);
    const documentTransitions = getDocumentTranstionsFixture({
      create: documents,
    });

    const stateTransition = new DocumentsBatchTransition({
      ownerId: getDocumentsFixture.ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    }, [dataContract]);

    const result = await createStateTransition(stateTransition.toJSON(), {
      fromJSON: true,
    });

    expect(result).to.be.instanceOf(DocumentsBatchTransition);
    expect(result.getTransitions().map((t) => t.toJSON())).to.have.deep.members(
      documentTransitions.map((t) => t.toJSON()),
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
      expect(e.getRawStateTransition()).to.equal(rawStateTransition);
    }
  });
});
