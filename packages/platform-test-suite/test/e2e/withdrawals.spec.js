const { expect } = require('chai');
const { Identifier } = require('@dashevo/wasm-dpp');

const {
  contractId: withdrawalsContractId,
} = require('@dashevo/withdrawals-contract/lib/systemIds');

const { contractId: masternodeRewardSharesContractId } = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');
const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../lib/waitForSTPropagated');

describe('Withdrawals', () => {
  let failed = false;
  let client;

  before(async () => {
    client = await createClientWithFundedWallet(
      10000000,
    );

    await client.platform.initialize();

    const withdrawalsContract = await client.platform.contracts.get(
      withdrawalsContractId,
    );

    client.getApps().set('withdrawals', {
      contractId: withdrawalsContractId,
      contract: withdrawalsContract,
    });
  });

  // Skip test if any prior test in this describe failed
  beforeEach(function beforeEach() {
    if (failed) {
      this.skip();
    }
  });

  afterEach(function afterEach() {
    failed = this.currentTest.state === 'failed';
  });

  after(async () => {
    if (client) {
      await client.disconnect();
    }
  });

  describe('Data Contract', () => {
    it('should exist', async () => {
      const existingDataContract = await client.platform.contracts.get(
        withdrawalsContractId,
      );

      expect(existingDataContract).to.exist();

      expect(existingDataContract.getId().toString()).to.equal(
        withdrawalsContractId,
      );
    });
  });
});
