const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');
const BlockExecutionState = require('../../../../lib/blockchainState/blockExecution/BlockExecutionState');

describe('BlockExecutionState', () => {
  let blockExecutionState;
  let dataContract;

  beforeEach(() => {
    blockExecutionState = new BlockExecutionState();
    dataContract = getDataContractFixture();
  });

  it('should add a Data Contract', async () => {
    expect(blockExecutionState.getDataContracts()).to.have.lengthOf(0);

    blockExecutionState.addDataContract(dataContract);
    const contracts = blockExecutionState.getDataContracts();

    expect(contracts).to.have.lengthOf(1);
    expect(contracts[0]).to.deep.equal(dataContract);
  });

  it('should reset state', async () => {
    blockExecutionState.addDataContract(dataContract);

    expect(blockExecutionState.getDataContracts()).to.have.lengthOf(1);

    blockExecutionState.reset();

    expect(blockExecutionState.getDataContracts()).to.have.lengthOf(0);
  });
});
