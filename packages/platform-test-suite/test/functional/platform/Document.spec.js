const getDataContractFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getDataContractFixture',
);
const getIdentityFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getIdentityFixture',
);

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');

describe('Platform', () => {
  describe('Document', () => {
    let client;
    let dataContractFixture;
    let identity;
    let document;

    before(async () => {
      client = await createClientWithFundedWallet();

      identity = await client.platform.identities.register(2);

      dataContractFixture = getDataContractFixture(identity.getId());

      await client.platform.contracts.broadcast(dataContractFixture, identity);

      client.apps.customContracts = {
        contractId: dataContractFixture.getId(),
        contract: dataContractFixture,
      };
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should fail to create new document with an unknown type', async () => {
      const newDocument = await client.platform.documents.create(
        'customContracts.niceDocument',
        identity,
        {
          name: 'anotherName',
        },
      );

      newDocument.type = 'unknownDocument';

      try {
        await client.platform.documents.broadcast({
          create: [newDocument],
        }, identity);

        expect.fail('should throw invalid argument error');
      } catch (e) {
        expect(e.message).to.satisfy(
          (msg) => msg.startsWith('StateTransition is invalid'),
        );
      }
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

      try {
        await client.platform.documents.broadcast({
          create: [document],
        }, unknownIdentity);
      } catch (e) {
        expect(e.message).to.equal(
          `Identity with ID ${unknownIdentity.getId()} is not associated with wallet, or it's not synced`,
        );
      }
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

      try {
        await client.platform.documents.broadcast({
          create: [secondDocument],
        }, identity);
        expect.fail('Error was not thrown');
      } catch (e) {
        const [error] = JSON.parse(e.metadata.get('errors'));
        expect(error.name).to.equal('DuplicateDocumentError');
        expect(error.indexDefinition).to.deep.equal({
          unique: true,
          properties: [
            { $ownerId: 'asc' },
            { firstName: 'desc' },
          ],
        });
      }
    });

    it('should be able to create new document', async () => {
      document = await client.platform.documents.create(
        'customContracts.niceDocument',
        identity,
        {
          name: 'myName',
        },
      );

      await client.platform.documents.broadcast({
        create: [document],
      }, identity);
    });

    it('should fetch created documents array', async () => {
      const [fetchedDocument] = await client.platform.documents.get(
        'customContracts.niceDocument',
        { where: [['$id', '==', document.getId()]] },
      );

      expect(document.toJSON()).to.deep.equal(fetchedDocument.toJSON());
    });
  });
});
