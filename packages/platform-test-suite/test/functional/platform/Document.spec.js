const { expect } = require('chai');

const getDataContractFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getDataContractFixture',
);

const getIdentityFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getIdentityFixture',
);

const { signStateTransition } = require('dash/build/src/SDK/Client/Platform/signStateTransition');

const InvalidDocumentTypeError = require('@dashevo/dpp/lib/errors/consensus/basic/document/InvalidDocumentTypeError');
const { StateTransitionBroadcastError } = require('dash/build/src/errors/StateTransitionBroadcastError');

const wait = require('../../../lib/wait');

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

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      dataContractFixture = getDataContractFixture(identity.getId());

      await client.platform.contracts.publish(dataContractFixture, identity);

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

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
        type: 'object',
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

      // mock validateBasic to skip validation in SDK
      this.sinon.stub(client.platform.dpp.stateTransition, 'validateBasic');

      client.platform.dpp.stateTransition.validateBasic.returns({
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

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause()).to.be.an.instanceOf(InvalidDocumentTypeError);
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

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

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
      expect(/Document \w* has duplicate unique properties \$ownerId, firstName with other documents/.test(broadcastError.message)).to.be.true();
      expect(broadcastError.code).to.be.equal(4009);
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

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }
    });

    it('should fetch created document', async () => {
      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      expect(fetchedDocument).to.exist();
      expect(document.toObject()).to.deep.equal(fetchedDocument.toObject());
      expect(fetchedDocument.getUpdatedAt().getTime())
        .to.be.equal(fetchedDocument.getCreatedAt().getTime());
    });

    it('should be able to fetch created document by created timestamp', async () => {
      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$createdAt', '==', document.getCreatedAt().getTime()]] },
      );

      expect(fetchedDocument).to.exist();
      expect(document.toObject()).to.deep.equal(fetchedDocument.toObject());
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

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      expect(fetchedDocument.get('firstName')).to.equal('updatedName');
      expect(fetchedDocument.getUpdatedAt().getTime())
        .to.be.greaterThan(fetchedDocument.getCreatedAt().getTime());
    });

    it('should be able to prove that a document was updated', async () => {
      const [storedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      storedDocument.set('firstName', 'updatedName');

      const documentsBatchTransition = await client.platform.documents.broadcast({
        replace: [storedDocument],
      }, identity);

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      documentsBatchTransition.transitions[0].data.firstName = 'nameToProve';
      documentsBatchTransition.transitions[0].updatedAt = new Date();
      documentsBatchTransition.transitions[0].revision += 1;

      const signedTransition = await signStateTransition(
        client.platform,
        documentsBatchTransition,
        identity,
      );

      const proof = await client.platform.broadcastStateTransition(signedTransition);

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      expect(proof.rootTreeProof).to.be.an.instanceof(Uint8Array);
      expect(proof.rootTreeProof.length).to.be.greaterThan(0);

      expect(proof.storeTreeProofs).to.exist();
      expect(proof.storeTreeProofs.documentsProof).to.be.an.instanceof(Uint8Array);
      expect(proof.storeTreeProofs.documentsProof.length).to.be.greaterThan(0);

      expect(proof.signatureLLMQHash).to.be.an.instanceof(Uint8Array);
      expect(proof.signatureLLMQHash.length).to.be.equal(32);

      expect(proof.signature).to.be.an.instanceof(Uint8Array);
      expect(proof.signature.length).to.be.equal(96);
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

      // Additional wait time to mitigate testnet latency
      if (process.env.NETWORK === 'testnet') {
        await wait(5000);
      }

      documentsBatchTransition.transitions[0].updatedAt = updatedAt;
      documentsBatchTransition.transitions[0].revision += 1;

      const signedTransition = await signStateTransition(
        client.platform,
        documentsBatchTransition,
        identity,
      );

      try {
        await client.platform.broadcastStateTransition(signedTransition);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.exist();
      expect(/Document \w* updatedAt timestamp .* are out of block time window from .* and .*/.test(broadcastError.message)).to.be.true();
      expect(broadcastError.code).to.be.equal(4008);
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
      expect(/Document \w* createdAt timestamp .* are out of block time window from .* and .*/.test(broadcastError.message)).to.be.true();
      expect(broadcastError.code).to.be.equal(4008);
    });
  });
});
