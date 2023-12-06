const { expect } = require('chai');
const { Identifier } = require('@dashevo/wasm-dpp');

const {
  contractId: withdrawalsContractId,
} = require('@dashevo/withdrawals-contract/lib/systemIds');

const { contractId: masternodeRewardSharesContractId } = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');
const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../lib/waitForSTPropagated');
const generateRandomIdentifier = require('../../lib/test/utils/generateRandomIdentifier');

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

  describe('Any Identity', () => {
    let identity;

    before(async () => {
      identity = await client.platform.identities.register(1000000);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();
    });

    it('should not be able to create withdrawals', async () => {
      const withdrawal = await client.platform.documents.create(
        'withdrawals.withdrawal',
        identity,
        {
          amount: 1000,
          coreFeePerByte: 1365,
          pooling: 0,
          outputScript: Buffer.alloc(23),
          status: 0,
        },
      );
      const stateTransition = client.platform.dpp.document.createStateTransition({
        create: [withdrawal],
      });

      stateTransition.setSignaturePublicKeyId(1);

      const account = await client.getWalletAccount();

      const { privateKey } = account.identities.getIdentityHDKeyById(
        identity.getId().toString(),
        1,
      );

      await stateTransition.sign(
        identity.getPublicKeyById(1),
        privateKey.toBuffer(),
      );

      try {
        await client.platform.documents.broadcast({
          create: [withdrawal],
        }, identity);

        expect.fail('should throw broadcast error');
      } catch (e) {
        expect(e.message).to.be.equal('Action is not allowed');
        expect(e.code).to.equal(4001);
      }
    });
  });
});
