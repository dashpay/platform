const DataContractCreateTransition = require(
  '../../../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition',
);

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const applyDataContractCreateTransitionFactory = require(
  '../../../../../lib/dataContract/stateTransition/DataContractCreateTransition/applyDataContractCreateTransitionFactory',
);

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const StateTransitionExecutionContext = require('../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('applyDataContractCreateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let stateRepositoryMock;
  let applyDataContractCreateTransition;
  let executionContext;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: Buffer.alloc(32),
    });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    applyDataContractCreateTransition = applyDataContractCreateTransitionFactory(
      stateRepositoryMock,
    );
  });

  it('should store a data contract from state transition in the repository', async () => {
    await applyDataContractCreateTransition(stateTransition);

    expect(stateRepositoryMock.storeDataContract).to.have.been.calledOnceWithExactly(
      stateTransition.getDataContract(),
      executionContext,
    );
  });
});
