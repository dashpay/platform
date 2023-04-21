const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('applyDataContractUpdateTransitionFactory', () => {
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

  beforeEach(() => {
    dataContract = getDataContractFixture();

    stateTransition = new DataContractUpdateTransition({
      dataContract: dataContract.toObject(),
      protocolVersion: protocolVersion.latestVersion,
    });

    executionContext = new StateTransitionExecutionContext();

    const stateRepositoryLike = {
      storeDataContract: async () => {
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
