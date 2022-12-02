const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('applyDataContractCreateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let applyDataContractCreateTransition;
  let executionContext;
  let DatacontractCreateTransition;
  let ApplyDataContractCreateTransition;
  let StateTransitionExecutionContext;

  before(async () => {
    ({
      DataContractCreateTransition, ApplyDataContractCreateTransition, StateTransitionExecutionContext,
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

    applyDataContractCreateTransition = new ApplyDataContractCreateTransition(stateRepositoryLike);
  });

  it('should store a data contract from state transition in the repository', async () => {
    await applyDataContractCreateTransition(stateTransition);
  });
});
