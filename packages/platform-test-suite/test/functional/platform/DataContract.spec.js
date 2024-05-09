const Dash = require('dash');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../../lib/waitForSTPropagated');

const {
  Errors: {
    StateTransitionBroadcastError,
  },
  PlatformProtocol: {
    IdentityNotFoundError,
    InvalidDataContractVersionError,
    IncompatibleDataContractSchemaError,
  },
} = Dash;

describe('Platform', () => {
  describe('Data Contract', function main() {
    this.timeout(700000);

    let client;
    let dataContractFixture;
    let identity;

    before(async () => {
      client = await createClientWithFundedWallet(35000000);

      // Looks like updating the contact and keeping history requires about
      // 7 million credits in fees. Investigate this further.
      identity = await client.platform.identities.register(30000000);
      const nextNonce = await client.platform
        .nonceManager.bumpIdentityNonce(identity.getId());
      dataContractFixture = await getDataContractFixture(nextNonce);
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should fail to create new data contract with unknown owner', async () => {
      // if no identity is specified
      // random is generated within the function
      dataContractFixture = await getDataContractFixture(1);

      let broadcastError;

      try {
        await client.platform.contracts.publish(dataContractFixture, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause().getCode()).to.equal(20000);
      expect(broadcastError.getCause()).to.be.an.instanceOf(IdentityNotFoundError);
    });

    it('should create new data contract with previously created identity as an owner', async () => {
      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const identityNonce = await client.platform.nonceManager
        .bumpIdentityNonce(identity.getId());
      dataContractFixture = await getDataContractFixture(identityNonce, identity.getId());

      await client.platform.contracts.publish(dataContractFixture, identity);
    });

    it('should be able to get newly created data contract', async () => {
      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      expect(fetchedDataContract).to.be.not.null();
      expect(dataContractFixture.toObject()).to.deep.equal(fetchedDataContract.toObject());
    });

    it('should not be able to update an existing data contract if version is incorrect', async () => {
      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

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
      expect(broadcastError.getCause().getCode()).to.equal(10212);
      expect(broadcastError.getCause()).to.be.an.instanceOf(InvalidDataContractVersionError);
    });

    // TODO(versioning): this test is not passing
    it('should not be able to update an existing data contract if schema is not backward compatible', async () => {
      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      const documentSchema = fetchedDataContract.getDocumentSchema('withByteArrays');
      delete documentSchema.properties.identifierField;
      fetchedDataContract.setDocumentSchema('withByteArrays', documentSchema);

      let broadcastError;

      try {
        await client.platform.contracts.update(fetchedDataContract, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause().getCode()).to.equal(10213);
      expect(broadcastError.getCause()).to.be.an.instanceOf(IncompatibleDataContractSchemaError);
    });

    it('should be able to update an existing data contract', async () => {
      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const fetchedDataContract = await client.platform.contracts.get(
        dataContractFixture.getId(),
      );

      const newDocumentType = 'myAwesomeDocument';

      // binary contract representation doesn't have a contract config in it,
      // and we set default value on deserialization, so we need to set it
      // here to avoid the error, as original contract has a non-default
      // value here
      fetchedDataContract.setConfig({
        canBeDeleted: false,
        readonly: false,
        keepsHistory: true,
        documentsKeepHistoryContractDefault: false,
        documentsMutableContractDefault: true,
      });
      fetchedDataContract.setDocumentSchema(newDocumentType, {
        type: 'object',
        indices: [
          {
            name: 'firstName',
            properties: [
              { firstName: 'asc' },
            ],
            unique: true,
          },
          {
            name: 'firstNameLastName',
            properties: [
              { firstName: 'asc' },
              { lastName: 'asc' },
            ],
            unique: true,
          },
        ],
        properties: {
          firstName: {
            type: 'string',
            maxLength: 63,
            position: 0,
          },
          lastName: {
            type: 'string',
            maxLength: 63,
            position: 1,
          },
        },
        required: ['firstName', '$createdAt', '$updatedAt', 'lastName'],
        additionalProperties: false,
      });

      await client.platform.contracts.update(fetchedDataContract, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      client.getApps().set('customContract', {
        contractId: fetchedDataContract.getId(),
        contract: fetchedDataContract,
      });

      const document = await client.platform.documents.create(
        `customContract.${newDocumentType}`,
        identity,
        {
          firstName: 'myName',
          lastName: 'myLastName',
        },
      );

      await client.platform.documents.broadcast({
        create: [document],
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const [fetchedDocument] = await client.platform.documents.get(
        `customContract.${newDocumentType}`,
        { where: [['firstName', '==', 'myName']] },
      );

      expect(fetchedDocument.getData()).to.deep.equal(
        {
          firstName: 'myName',
          lastName: 'myLastName',
        },
      );

      const contractHistory = await client.platform.contracts
        .history(dataContractFixture.getId(), 0, 10, 0);

      // By default, history is not really sorted, since it's a map
      const historyPairs = Object.entries(contractHistory);
      historyPairs.sort((a, b) => a[0] - b[0]);

      expect(historyPairs).to.have.lengthOf(2);

      const [originalContractDate, originalContract] = Object.entries(contractHistory)[0];
      expect(originalContract.toObject()).to.be.deep.equal(dataContractFixture.toObject());

      const [updatedContractDate, updatedContract] = Object.entries(contractHistory)[1];
      // Version is updated separately inside SDK on a cloned contract, so we need to update it
      //  here manually to compare
      fetchedDataContract.incrementVersion();
      expect(updatedContract.toObject()).to.be.deep.equal(fetchedDataContract.toObject());

      expect(Number(updatedContractDate)).to.be.greaterThan(Number(originalContractDate));
    });
  });
});
