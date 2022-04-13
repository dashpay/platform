const DataContractUpdateTransition = require(
  '../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/DataContractUpdateTransition',
);

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const applyDataContractUpdateTransitionFactory = require(
  '../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/applyDataContractUpdateTransitionFactory',
);

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');

describe('applyDataContractUpdateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let stateRepositoryMock;
  let applyDataContractUpdateTransition;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    stateTransition = new DataContractUpdateTransition({
      dataContract: dataContract.toObject(),
    });

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    applyDataContractUpdateTransition = applyDataContractUpdateTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should store a data contract from state transition in the repository', async () => {
    await applyDataContractUpdateTransition(stateTransition);

    expect(stateRepositoryMock.storeDataContract).to.have.been.calledOnceWithExactly(
      stateTransition.getDataContract(),
    );
  });
});
