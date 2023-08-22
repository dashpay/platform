const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../../..');
const { getLatestProtocolVersion } = require('../../../../..');

describe.skip('applyDataContractUpdateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let executionContext;
  let dataContractStored;
  let factory;
  let DataContractUpdateTransition;
  let ApplyDataContractUpdateTransition;
  let StateTransitionExecutionContext;

  before(async () => {
    ({
      DataContractUpdateTransition,
      ApplyDataContractUpdateTransition,
      StateTransitionExecutionContext,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    dataContract = await getDataContractFixture();

    stateTransition = new DataContractUpdateTransition({
      dataContract: dataContract.toObject(),
      protocolVersion: getLatestProtocolVersion(),
    });

    executionContext = new StateTransitionExecutionContext();

    const stateRepositoryLike = {
      updateDataContract: async () => {
        dataContractStored = true;
      },
    };

    factory = new ApplyDataContractUpdateTransition(stateRepositoryLike);

    dataContractStored = false;
  });

  it('should store a data contract from state transition in the repository', async () => {
    await factory.applyDataContractUpdateTransition(stateTransition, executionContext);
    expect(dataContractStored).to.be.true();
  });
});
