const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
const { default: loadWasmDpp } = require('../../../../..');
const { getLatestProtocolVersion } = require('../../../../..');

describe.skip('applyDataContractCreateTransitionFactory', () => {
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

  beforeEach(async () => {
    dataContract = await getDataContractFixture();

    stateTransition = new DataContractCreateTransition({
      protocolVersion: getLatestProtocolVersion(),
      dataContract: dataContract.toObject(),
      entropy: Buffer.alloc(32),
    });

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
