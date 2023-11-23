const crypto = require('crypto');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');
const { default: loadWasmDpp } = require('../../..');
const {
  DashPlatformProtocol,
  DataContractCreateTransition,
  DocumentsBatchTransition,
  InvalidStateTransitionTypeError,
} = require('../../..');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

describe.skip('createStateTransitionFactory', () => {
  let createStateTransition;
  let stateRepositoryMock;
  let dataContract;
  let dpp;
  let blsAdapter;

  before(async () => {
    await loadWasmDpp();
  });

  beforeEach(async function beforeEach() {
    blsAdapter = await getBlsAdapterMock();
    stateRepositoryMock = createStateRepositoryMock(this.sinon);

    dpp = new DashPlatformProtocol(
      blsAdapter,
      stateRepositoryMock,
      { generate: () => crypto.randomBytes(32) },
      1,
    );

    dataContract = await getDataContractFixture();
    stateRepositoryMock.fetchDataContract.resolves(dataContract);
    createStateTransition = (st) => dpp.stateTransition.createFromObject(
      st,
      { skipValidation: true },
    );
  });

  it('should return DataContractCreateTransition if type is DATA_CONTRACT_CREATE', async () => {
    const stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
      protocolVersion: 1,
    });

    const result = await createStateTransition(stateTransition.toObject());

    expect(result).to.be.instanceOf(DataContractCreateTransition);
    expect(result.getDataContract().toObject()).to.deep.equal(dataContract.toObject());
  });

  it('should return DocumentsBatchTransition if type is DOCUMENTS', async () => {
    const documents = await getDocumentsFixture(dataContract);
    const ownerId = documents[0].getOwnerId();
    const documentTransitions = getDocumentTransitionsFixture({
      create: documents,
    });

    const stateTransition = new DocumentsBatchTransition({
      ownerId,
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
      type: 253,
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
