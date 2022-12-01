const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const applyDataContractCreateTransitionFactory = require(
  '@dashevo/dpp/lib/dataContract/stateTransition/DataContractCreateTransition/applyDataContractCreateTransitionFactory',
);

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const StateTransitionExecutionContext = require('@dashevo/dpp/lib/stateTransition/StateTransitionExecutionContext');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('applyDataContractCreateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let stateRepositoryMock;
  let applyDataContractCreateTransition;
  let executionContext;
  let DatacontractCreateTransition;

  before(async () => {
    ({
      DataContractCreateTransition,
    } = await loadWasmDpp());   
  });

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

    expect(stateRepositoryMock.createDataContract).to.have.been.calledOnceWithExactly(
      stateTransition.getDataContract(),
      executionContext,
    );
  });
});
