const { expect } = require('chai');

const getDataContractFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getDataContractFixture',
);
const getIdentityFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getIdentityFixture',
);

const { signStateTransition } = require('dash/build/src/SDK/Client/Platform/signStateTransition');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Platform', () => {
  describe('Document', function main() {
    this.timeout(700000);

    let client;
    let dataContractFixture;
    let identity;
    let document;

    before(async () => {
      client = await createClientWithFundedWallet();

      identity = await client.platform.identities.register(10);

      dataContractFixture = getDataContractFixture(identity.getId());

      await client.platform.contracts.broadcast(dataContractFixture, identity);

      client.getApps().set('customContracts', {
        contractId: dataContractFixture.getId(),
        contract: dataContractFixture,
      });
    });

    beforeEach(() => {
      dataContractFixture = getDataContractFixture(identity.getId());
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should fail to create new document with an unknown type', async function it() {
      // Add undefined document type for
      client.getApps().get('customContracts').contract.documents.undefinedType = {
        properties: {
          name: {
            type: 'string',
          },
        },
        additionalProperties: false,
      };

      const newDocument = await client.platform.documents.create(
        'customContracts.undefinedType',
        identity,
        {
          name: 'anotherName',
        },
      );

      // mock validateStructure to skip validation in SDK
      this.sinon.stub(client.platform.dpp.stateTransition, 'validateStructure');

      client.platform.dpp.stateTransition.validateStructure.returns({
        isValid: () => true,
      });

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [newDocument],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.be.equal(3);
      expect(broadcastError.message).to.be.equal('State Transition is invalid: InvalidDocumentTypeError: Contract doesn\'t contain type undefinedType');
      const [error] = broadcastError.data.errors;
      expect(error.name).to.equal('InvalidDocumentTypeError');
    });

    it('should fail to create a new document with an unknown owner', async () => {
      const unknownIdentity = getIdentityFixture();

      document = await client.platform.documents.create(
        'customContracts.niceDocument',
        unknownIdentity,
        {
          name: 'myName',
        },
      );

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [document],
        }, unknownIdentity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.equal(
        `Identity with ID ${unknownIdentity.getId()} is not associated with wallet, or it's not synced`,
      );
    });

    it('should fail to create a document that violates unique index constraint', async () => {
      const sharedDocumentData = {
        firstName: 'Some First Name',
      };

      const firstDocument = await client.platform.documents.create(
        'customContracts.indexedDocument',
        identity,
        {
          ...sharedDocumentData,
          lastName: 'Some Last Name',
        },
      );

      await client.platform.documents.broadcast({
        create: [firstDocument],
      }, identity);

      const secondDocument = await client.platform.documents.create(
        'customContracts.indexedDocument',
        identity,
        {
          ...sharedDocumentData,
          lastName: 'Other Last Name',
        },
      );

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [secondDocument],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.be.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DuplicateDocumentError: Duplicate Document found');

      const [error] = broadcastError.data.errors;
      expect(error.name).to.equal('DuplicateDocumentError');
      expect(error.indexDefinition).to.deep.equal({
        unique: true,
        properties: [
          { $ownerId: 'asc' },
          { firstName: 'desc' },
        ],
      });
    });

    it('should be able to create new document', async () => {
      document = await client.platform.documents.create(
        'customContracts.indexedDocument',
        identity,
        {
          firstName: 'myName',
          lastName: 'lastName',
        },
      );

      await client.platform.documents.broadcast({
        create: [document],
      }, identity);
    });

    it('should fetch created document', async () => {
      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      expect(document.toJSON()).to.deep.equal(fetchedDocument.toJSON());
      expect(fetchedDocument.getUpdatedAt().getTime())
        .to.be.equal(fetchedDocument.getCreatedAt().getTime());
    });

    it('should be able to fetch created document by created timestamp', async () => {
      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$createdAt', '==', document.getCreatedAt().getTime()]] },
      );

      expect(document.toJSON()).to.deep.equal(fetchedDocument.toJSON());
    });

    it('should be able to update document', async () => {
      const [storedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      storedDocument.set('firstName', 'updatedName');

      await client.platform.documents.broadcast({
        replace: [storedDocument],
      }, identity);

      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      expect(fetchedDocument.get('firstName')).to.equal('updatedName');
      expect(fetchedDocument.getUpdatedAt().getTime())
        .to.be.greaterThan(fetchedDocument.getCreatedAt().getTime());
    });

    it('should fail to update document with timestamp in violated time frame', async () => {
      const [storedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      const updatedAt = storedDocument.getUpdatedAt();

      updatedAt.setMinutes(updatedAt.getMinutes() - 10);

      let broadcastError;

      const documentsBatchTransition = await client.platform.documents.broadcast({
        replace: [storedDocument],
      }, identity);

      documentsBatchTransition.transitions[0].updatedAt = updatedAt;
      documentsBatchTransition.transitions[0].revision += 1;
      const signedTransition = await signStateTransition(
        client.platform, documentsBatchTransition, identity,
      );

      try {
        await client.platform.broadcastStateTransition(signedTransition);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.message).to.be.equal('Invalid state transition: DocumentTimestampWindowViolationError: Document updatedAt timestamp are out of block time window');
      expect(broadcastError.code).to.be.equal(2);

      const [error] = broadcastError.data.errors;
      expect(error.name).to.equal('DocumentTimestampWindowViolationError');
    });

    it('should fail to create a new document with timestamp in violated time frame', async () => {
      document = await client.platform.documents.create(
        'customContracts.indexedDocument',
        identity,
        {
          firstName: 'myName',
          lastName: 'lastName',
        },
      );

      const createdAt = document.getCreatedAt();

      createdAt.setMinutes(createdAt.getMinutes() - 10);

      document.setUpdatedAt(createdAt);

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [document],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(broadcastError.code).to.be.equal(2);
      expect(broadcastError.message).to.be.equal('Invalid state transition: DocumentTimestampWindowViolationError: Document createdAt timestamp are out of block time window and 2 more');

      const [error] = broadcastError.data.errors;
      expect(error.name).to.equal('DocumentTimestampWindowViolationError');
    });
  });
});
