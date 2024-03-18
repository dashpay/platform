const Dash = require('dash');
const { expect } = require('chai');

const { signStateTransition } = require('dash/build/SDK/Client/Platform/signStateTransition');

const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../../lib/waitForSTPropagated');

const {
  Errors: {
    StateTransitionBroadcastError,
  },
  PlatformProtocol: {
    InvalidDocumentTypeError,
  },
} = Dash;

const getDocumentObject = (document) => {
  const documentObject = document.toObject();

  // Delete createdAt and updatedAt fields because they could vary slightly
  delete documentObject.$createdAt;
  delete documentObject.$updatedAt;

  return documentObject;
};

describe('Platform', () => {
  describe('Document', function main() {
    this.timeout(700000);

    let client;
    let dataContractFixture;
    let identity;
    let document;

    before(async () => {
      client = await createClientWithFundedWallet(1010000);

      identity = await client.platform.identities.register(1000000);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const identityNonce = await client.platform
        .nonceManager.bumpIdentityNonce(identity.getId());
      dataContractFixture = await getDataContractFixture(identityNonce, identity.getId());

      await client.platform.contracts.publish(dataContractFixture, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      client.getApps()
        .set('customContracts', {
          contractId: dataContractFixture.getId(),
          contract: dataContractFixture,
        });
    });

    beforeEach(async () => {
      const identityNonce = await client.platform
        .nonceManager.bumpIdentityNonce(identity.getId());
      dataContractFixture = await getDataContractFixture(identityNonce, identity.getId());
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should fail to create new document with an unknown type', async () => {
      // Add undefined document type for
      client.getApps()
        .get('customContracts')
        .contract
        .setDocumentSchema('undefinedType', {
          type: 'object',
          properties: {
            name: {
              type: 'string',
              position: 0,
            },
          },
          additionalProperties: false,
        });

      const newDocument = await client.platform.documents.create(
        'customContracts.undefinedType',
        identity,
        {
          name: 'anotherName',
        },
      );

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [newDocument],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError)
        .to
        .be
        .an
        .instanceOf(StateTransitionBroadcastError);
      expect(broadcastError.getCause())
        .to
        .be
        .an
        .instanceOf(InvalidDocumentTypeError);
    });

    it('should fail to create a new document with an unknown owner', async () => {
      const unknownIdentity = await getIdentityFixture();

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

      expect(broadcastError)
        .to
        .exist();
      expect(broadcastError.message)
        .to
        .equal(
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
      await waitForSTPropagated();

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

      expect(broadcastError)
        .to
        .exist();
      expect(broadcastError.code)
        .to
        .be
        .equal(4009);
      expect(broadcastError.message)
        .to
        .match(/Document \w* has duplicate unique properties \["\$ownerId", "firstName"] with other documents/);
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
      await waitForSTPropagated();
    });

    it('should fetch created document', async () => {
      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      expect(fetchedDocument)
        .to
        .exist();

      expect(fetchedDocument.getUpdatedAt())
        .to.be.greaterThanOrEqual(document.getUpdatedAt());
      expect(fetchedDocument.getCreatedAt())
        .to.be.greaterThanOrEqual(document.getCreatedAt());

      expect(getDocumentObject(document)).to.deep.equal(getDocumentObject(fetchedDocument));
      expect(fetchedDocument.getUpdatedAt()).to.be.deep.equal(fetchedDocument.getCreatedAt());
    });

    it('should be able to fetch created document by created timestamp', async () => {
      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        {
          where: [['$createdAt', '>', document.getCreatedAt()
            .getTime()]],
          orderBy: [['$createdAt', 'desc']],
        },
      );

      expect(fetchedDocument).to.exist();
      expect(getDocumentObject(document)).to.deep.equal(getDocumentObject(fetchedDocument));
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
      await waitForSTPropagated();

      const [fetchedDocument] = await client.platform
        .documents.get(
          'customContracts.indexedDocument',
          { where: [['$id', '==', document.getId()]] },
        );

      expect(fetchedDocument.get('firstName'))
        .to
        .equal('updatedName');
      expect(fetchedDocument.getUpdatedAt())
        .to
        .be
        .greaterThan(fetchedDocument.getCreatedAt());
    });

    it.skip('should be able to prove that a document was updated', async () => {
      const [storedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      storedDocument.set('firstName', 'updatedName');

      const documentsBatchTransition = await client.platform.documents.broadcast({
        replace: [storedDocument],
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      documentsBatchTransition.transitions[0].data.firstName = 'nameToProve';
      documentsBatchTransition.transitions[0].updatedAt = new Date();
      documentsBatchTransition.transitions[0].revision += 1;
      const signedTransition = await signStateTransition(
        client.platform,
        documentsBatchTransition,
        identity,
        1,
      );

      const proof = await client.platform.broadcastStateTransition(signedTransition);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      expect(proof.rootTreeProof)
        .to
        .be
        .an
        .instanceof(Uint8Array);
      expect(proof.rootTreeProof.length)
        .to
        .be
        .greaterThan(0);

      expect(proof.storeTreeProofs)
        .to
        .exist();
      expect(proof.storeTreeProofs.documentsProof)
        .to
        .be
        .an
        .instanceof(Uint8Array);
      expect(proof.storeTreeProofs.documentsProof.length)
        .to
        .be
        .greaterThan(0);

      expect(proof.quorumHash)
        .to
        .be
        .an
        .instanceof(Uint8Array);
      expect(proof.quorumHash.length)
        .to
        .be
        .equal(32);

      expect(proof.signature)
        .to
        .be
        .an
        .instanceof(Uint8Array);
      expect(proof.signature.length)
        .to
        .be
        .equal(96);

      expect(proof.round)
        .to
        .be
        .a('number');
      expect(proof.round)
        .to
        .be
        .greaterThanOrEqual(0);
    });

    it('should be able to delete a document', async () => {
      await client.platform.documents.broadcast({
        delete: [document],
      }, identity);

      await waitForSTPropagated();

      const [storedDocument] = await client.platform.documents.get(
        'customContracts.indexedDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      expect(storedDocument)
        .to
        .not
        .exist();
    });
  });
});
