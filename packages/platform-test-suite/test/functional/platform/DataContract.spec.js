const getDataContractFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getDataContractFixture',
);

const IdentityNotFoundError = require('@dashevo/dpp/lib/errors/consensus/signature/IdentityNotFoundError');
const { StateTransitionBroadcastError } = require('dash/build/src/errors/StateTransitionBroadcastError');

const wait = require('../../../lib/wait');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Platform', () => {
  describe('Data Contract', function main() {
    this.timeout(700000);

    let client;
    let dataContractFixture;
    let identity;

    before(async () => {
      client = await createClientWithFundedWallet();

      identity = await client.platform.identities.register(3);
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should fail to create new data contract with unknown owner', async () => {
      // if no identity is specified
      // random is generated within the function
      dataContractFixture = getDataContractFixture();

      let broadcastError;

      try {
        await client.platform.contracts.publish(dataContractFixture, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause()).to.be.an.instanceOf(IdentityNotFoundError);
    });

    it('should create new data contract with previously created identity as an owner', async () => {
      dataContractFixture = getDataContractFixture(identity.getId());

      await client.platform.contracts.publish(dataContractFixture, identity);
    });

    it('should be able to get newly created data contract', async () => {
      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      const fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      expect(fetchedDataContract).to.be.not.null();
      expect(dataContractFixture.toJSON()).to.deep.equal(fetchedDataContract.toJSON());
    });
  });
});
