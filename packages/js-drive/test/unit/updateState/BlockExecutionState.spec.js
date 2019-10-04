const BlockExecutionState = require('../../../lib/updateState/BlockExecutionState');
const getSVContractFixture = require('../../../lib/test/fixtures/getSVContractFixture');

describe('BlockExecutionState', () => {
  let blockExecutionState;
  let contract;

  beforeEach(() => {
    blockExecutionState = new BlockExecutionState();
    contract = getSVContractFixture();
  });

  it('should add contract', async () => {
    expect(blockExecutionState.getContracts()).to.have.lengthOf(0);

    blockExecutionState.addContract(contract);
    const contracts = blockExecutionState.getContracts();

    expect(contracts).to.have.lengthOf(1);
    expect(contracts[0]).to.deep.equal(contract);
  });

  it('should clear contracts', async () => {
    blockExecutionState.addContract(contract);

    expect(blockExecutionState.getContracts()).to.have.lengthOf(1);

    blockExecutionState.clearContracts();

    expect(blockExecutionState.getContracts()).to.have.lengthOf(0);
  });
});
