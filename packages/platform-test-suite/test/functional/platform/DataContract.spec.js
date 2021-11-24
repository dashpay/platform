const getDataContractFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getDataContractFixture',
);

const IdentityNotFoundError = require('@dashevo/dpp/lib/errors/consensus/signature/IdentityNotFoundError');
const { StateTransitionBroadcastError } = require('dash/build/src/errors/StateTransitionBroadcastError');

const InvalidDataContractVersionError = require('@dashevo/dpp/lib/errors/consensus/state/dataContract/InvalidDataContractVersionError');
const IncompatibleDataContractSchemaError = require('@dashevo/dpp/lib/errors/consensus/state/dataContract/IncompatibleDataContractSchemaError');

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

      identity = await client.platform.identities.register(5);
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

    it('should not be able to update an existing data contract if version is incorrect', async () => {
      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      const fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      fetchedDataContract.setVersion(fetchedDataContract.getVersion() + 2);

      let broadcastError;

      try {
        await client.platform.contracts.update(fetchedDataContract, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause()).to.be.an.instanceOf(InvalidDataContractVersionError);
    });

    it('should not be able to update an existing data contract if schema is not backward compatible', async () => {
      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      const fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      const documentSchema = fetchedDataContract.getDocumentSchema('withByteArrays');
      delete documentSchema.properties.identifierField;

      fetchedDataContract.setVersion(fetchedDataContract.getVersion() + 1);

      let broadcastError;

      try {
        await client.platform.contracts.update(fetchedDataContract, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause()).to.be.an.instanceOf(IncompatibleDataContractSchemaError);
    });

    it('should be able to update an existing data contract', async () => {
      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      let fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      const documentSchema = fetchedDataContract.getDocumentSchema('withByteArrays');
      documentSchema.properties.anotherOptionalField = {
        type: 'integer',
        minimum: 1,
      };

      fetchedDataContract.setVersion(fetchedDataContract.getVersion() + 1);

      await client.platform.contracts.update(fetchedDataContract, identity);

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      expect(fetchedDataContract.getDocumentSchema('withByteArrays').properties.anotherOptionalField).to.deep.equal(
        {
          type: 'integer',
          minimum: 1,
        },
      );
    });
  });
});
