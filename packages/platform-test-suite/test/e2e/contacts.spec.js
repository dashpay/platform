const Identity = require('@dashevo/dpp/lib/identity/Identity');
const Identifier = require('@dashevo/dpp/lib/Identifier');

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');

describe('e2e', () => {
  describe('Contacts', function contacts() {
    this.timeout(950000);

    let dataContract;

    let bobClient;
    let aliceClient;

    let bobIdentity;
    let bobContactRequest;
    let aliceIdentity;
    let aliceProfile;
    let aliceContactAcceptance;

    let dataContractDocumentSchemas;

    before(() => {
      dataContractDocumentSchemas = {
        profile: {
          indices: [
            { properties: [{ $ownerId: 'asc' }], unique: true },
          ],
          properties: {
            avatarUrl: {
              type: 'string',
              format: 'url',
              maxLength: 255,
            },
            about: {
              type: 'string',
              maxLength: 255,
            },
          },
          required: ['avatarUrl', 'about'],
          additionalProperties: false,
        },
        contact: {
          indices: [
            { properties: [{ $ownerId: 'asc' }, { toUserId: 'asc' }], unique: true },
          ],
          properties: {
            toUserId: {
              type: 'array',
              byteArray: true,
              contentMediaType: Identifier.MEDIA_TYPE,
              minItems: 32,
              maxItems: 32,
            },
            publicKey: {
              type: 'array',
              byteArray: true,
              maxItems: 33,
            },
          },
          required: ['toUserId', 'publicKey'],
          additionalProperties: false,
        },
      };
    });

    after(async () => {
      if (bobClient) {
        await bobClient.disconnect();
      }

      if (aliceClient) {
        await aliceClient.disconnect();
      }
    });

    describe('Bob', () => {
      it('should create user wallet and identity', async () => {
        // Create Bob wallet
        bobClient = await createClientWithFundedWallet();

        bobIdentity = await bobClient.platform.identities.register(10);

        expect(bobIdentity).to.be.instanceOf(Identity);
      });

      it('should publish "Contacts" data contract', async () => {
        // 1. Create and broadcast data contract
        dataContract = await bobClient.platform.contracts.create(
          dataContractDocumentSchemas, bobIdentity,
        );

        await bobClient.platform.contracts.broadcast(dataContract, bobIdentity);

        bobClient.getApps().set('contacts', {
          contractId: dataContract.getId(),
          contract: dataContract,
        });

        // 2. Fetch and check data contract
        const fetchedDataContract = await bobClient.platform.contracts.get(
          dataContract.getId(),
        );

        expect(fetchedDataContract.toJSON()).to.be.deep.equal(dataContract.toJSON());
      });

      it('should create profile in "Contacts" app', async () => {
        // 1. Create and broadcast profile
        const profile = await bobClient.platform.documents.create('contacts.profile', bobIdentity, {
          avatarUrl: 'http://test.com/bob.jpg',
          about: 'This is story about me',
        });

        await bobClient.platform.documents.broadcast({
          create: [profile],
        }, bobIdentity);

        // 2. Fetch and compare profiles
        const [fetchedProfile] = await bobClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', profile.getId()]] },
        );

        expect(fetchedProfile.toJSON()).to.be.deep.equal(profile.toJSON());
      });
    });

    describe('Alice', () => {
      it('should create user wallet and identity', async () => {
        // Create Alice wallet
        aliceClient = await createClientWithFundedWallet();

        aliceClient.getApps().set('contacts', {
          contractId: dataContract.getId(),
          contract: dataContract,
        });

        aliceIdentity = await aliceClient.platform.identities.register(10);

        expect(aliceIdentity).to.be.instanceOf(Identity);
      });

      it('should create profile in "Contacts" app', async () => {
        // 1. Create and broadcast profile
        aliceProfile = await aliceClient.platform.documents.create('contacts.profile', aliceIdentity, {
          avatarUrl: 'http://test.com/alice.jpg',
          about: 'I am Alice',
        });

        await aliceClient.platform.documents.broadcast({
          create: [aliceProfile],
        }, aliceIdentity);

        // 2. Fetch and compare profile
        const [fetchedProfile] = await aliceClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', aliceProfile.getId()]] },
        );

        expect(fetchedProfile.toJSON()).to.be.deep.equal(aliceProfile.toJSON());
      });

      it('should be able to update her profile', async () => {
        // 1. Update profile document
        aliceProfile.set('avatarUrl', 'http://test.com/alice2.jpg');

        // 2. Broadcast change
        await aliceClient.platform.documents.broadcast({
          replace: [aliceProfile],
        }, aliceIdentity);

        // 3. Fetch and compare profile
        const [fetchedProfile] = await aliceClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', aliceProfile.getId()]] },
        );

        expect(fetchedProfile.toJSON()).to.be.deep.equal({
          ...aliceProfile.toJSON(),
          $revision: 2,
        });
      });
    });

    describe('Bob', () => {
      it('should be able to send contact request', async () => {
        // 1. Create and broadcast contact document
        bobContactRequest = await bobClient.platform.documents.create('contacts.contact', bobIdentity, {
          toUserId: aliceIdentity.getId(),
          publicKey: bobIdentity.getPublicKeyById(0).getData(),
        });

        await bobClient.platform.documents.broadcast({
          create: [bobContactRequest],
        }, bobIdentity);

        // 2. Fetch and compare contacts
        const [fetchedContactRequest] = await bobClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', bobContactRequest.getId()]] },
        );

        expect(fetchedContactRequest.toJSON()).to.be.deep.equal(bobContactRequest.toJSON());
      });
    });

    describe('Alice', () => {
      it('should be able to approve contact request', async () => {
        // 1. Create and broadcast contact approval document
        aliceContactAcceptance = await aliceClient.platform.documents.create(
          'contacts.contact', aliceIdentity, {
            toUserId: bobIdentity.getId(),
            publicKey: aliceIdentity.getPublicKeyById(0).getData(),
          },
        );

        await aliceClient.platform.documents.broadcast({
          create: [aliceContactAcceptance],
        }, aliceIdentity);

        // 2. Fetch and compare contacts
        const [fetchedAliceContactAcceptance] = await aliceClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', aliceContactAcceptance.getId()]] },
        );

        expect(fetchedAliceContactAcceptance.toJSON()).to.be.deep.equal(
          aliceContactAcceptance.toJSON(),
        );
      });

      it('should be able to remove contact approval', async () => {
        // 1. Broadcast document deletion
        await aliceClient.platform.documents.broadcast({
          delete: [aliceContactAcceptance],
        }, aliceIdentity);

        // 2. Fetch contact documents and check it does not exists
        const [fetchedAliceContactAcceptance] = await aliceClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', aliceContactAcceptance.getId()]] },
        );

        expect(fetchedAliceContactAcceptance).to.not.exist();
      });
    });
  });
});
