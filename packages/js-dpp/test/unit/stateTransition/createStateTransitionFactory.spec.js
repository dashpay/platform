const createStateTransitionFactory = require('../../../lib/stateTransition/createStateTransitionFactory');

const DataContractStateTransition = require('../../../lib/dataContract/stateTransition/DataContractStateTransition');

const stateTransitionTypes = require('../../../lib/stateTransition/stateTransitionTypes');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const InvalidStateTransitionTypeError = require('../../../lib/errors/InvalidStateTransitionTypeError');

describe('createStateTransitionFactory', () => {
  let createDataContractMock;
  let createStateTransition;

  beforeEach(function beforeEach() {
    createDataContractMock = this.sinonSandbox.stub();

    createStateTransition = createStateTransitionFactory(
      createDataContractMock,
    );
  });

  it('should return DataContractStateTransition if type is DATA_CONTRACT', () => {
    const dataContract = getDataContractFixture();
    const rawDataContract = dataContract.toJSON();

    const rawStateTransition = {
      type: stateTransitionTypes.DATA_CONTRACT,
      dataContract: rawDataContract,
    };

    createDataContractMock.returns(dataContract);

    const result = createStateTransition(rawStateTransition);

    expect(result).to.be.instanceOf(DataContractStateTransition);
    expect(result.getDataContract()).to.equal(dataContract);

    expect(createDataContractMock).to.be.calledOnceWith(rawDataContract);
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
