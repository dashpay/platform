const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('applyDataContractCreateTransitionFactory', () => {
  let stateTransition;
  let dataContract;
  let factory;
  let DataContractCreateTransition;
  let ApplyDataContractCreateTransition;

  let dataContractStored;

  before(async () => {
    ({
      DataContractCreateTransition,
      ApplyDataContractCreateTransition,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    dataContract = getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: protocolVersion.latestVersion,
      dataContract: dataContract.toObject(),
      entropy: Buffer.alloc(32),
    });

    const stateRepositoryLike = {
      storeDataContract: async () => {
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
