const createStateTransitionFactory = require('../../../lib/stateTransition/createStateTransitionFactory');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition');
const DocumentsBatchTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTranstionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const InvalidStateTransitionTypeError = require('../../../lib/errors/InvalidStateTransitionTypeError');

describe('createStateTransitionFactory', () => {
  let createStateTransition;

  beforeEach(() => {
    createStateTransition = createStateTransitionFactory();
  });

  it('should return DataContractCreateTransition if type is DATA_CONTRACT_CREATE', () => {
    const dataContract = getDataContractFixture();

    const stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toJSON(),
      entropy: dataContract.getEntropy(),
    });

    const result = createStateTransition(stateTransition.toJSON());

    expect(result).to.be.instanceOf(DataContractCreateTransition);
    expect(result.getDataContract().toJSON()).to.deep.equal(dataContract.toJSON());
  });

  it('should return DocumentsBatchTransition if type is DOCUMENTS', () => {
    const documentTransitions = getDocumentTranstionsFixture();

    const stateTransition = new DocumentsBatchTransition({
      ownerId: getDocumentsFixture.ownerId,
      contractId: getDocumentsFixture.dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });

    const result = createStateTransition(stateTransition.toJSON());

    expect(result).to.be.instanceOf(DocumentsBatchTransition);
    expect(result.getTransitions()).to.deep.equal(documentTransitions);
  });

  it('should throw InvalidStateTransitionTypeError if type is invalid', () => {
    const rawStateTransition = {
      type: 666,
    };

    try {
      createStateTransition(rawStateTransition);

      expect.fail('InvalidStateTransitionTypeError is not thrown');
    } catch (e) {
      expect(e).to.be.an.instanceOf(InvalidStateTransitionTypeError);
      expect(e.getRawStateTransition()).to.equal(rawStateTransition);
    }
  });
});
