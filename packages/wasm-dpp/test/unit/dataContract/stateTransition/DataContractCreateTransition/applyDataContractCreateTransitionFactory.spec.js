const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const { default: loadWasmDpp } = require('../../../../..');
const { getLatestProtocolVersion } = require('../../../../..');

describe('applyDataContractCreateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let factory;
  let executionContext;
  let DataContractCreateTransition;
  let ApplyDataContractCreateTransition;
  let StateTransitionExecutionContext;

  let dataContractStored;

  before(async () => {
    ({
      DataContractCreateTransition,
      ApplyDataContractCreateTransition,
      StateTransitionExecutionContext,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: getLatestProtocolVersion(),
      dataContract: dataContract.toObject(),
      entropy: Buffer.alloc(32),
    });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    const stateRepositoryLike = {
      createDataContract: async () => {
        dataContractStored = true;
      },
    };

    factory = new ApplyDataContractCreateTransition(stateRepositoryLike);

    dataContractStored = false;
  });

  it('should store a data contract from state transition in the repository', async () => {
    await factory.applyDataContractCreateTransition(stateTransition);
    expect(dataContractStored).to.be.true();
  });
});
