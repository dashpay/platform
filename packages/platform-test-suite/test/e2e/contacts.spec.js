const Dash = require('dash');

const {
  PlatformProtocol: {
    Identifier,
    IdentityPublicKey,
    IdentityPublicKeyWithWitness,
  },
} = Dash;

const createClientWithFundedWallet = require('../../lib/test/createClientWithFundedWallet');
const waitForSTPropagated = require('../../lib/waitForSTPropagated');

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
          type: 'object',
          indices: [
            {
              name: 'ownerId',
              properties: [{ $ownerId: 'asc' }],
              unique: true,
            },
          ],
          properties: {
            avatarUrl: {
              type: 'string',
              format: 'uri',
              maxLength: 255,
              position: 0,
            },
            about: {
              type: 'string',
              maxLength: 255,
              position: 1,
            },
          },
          required: ['avatarUrl', 'about'],
          additionalProperties: false,
        },
        contact: {
          type: 'object',
          requiresIdentityEncryptionBoundedKey: 2,
          requiresIdentityDecryptionBoundedKey: 2,
          indices: [
            {
              name: 'onwerIdToUserId',
              properties: [
                { $ownerId: 'asc' },
                { toUserId: 'asc' },
              ],
              unique: true,
            },
          ],
          properties: {
            toUserId: {
              type: 'array',
              byteArray: true,
              contentMediaType: Identifier.MEDIA_TYPE,
              minItems: 32,
              maxItems: 32,
              position: 0,
            },
            publicKey: {
              type: 'array',
              byteArray: true,
              maxItems: 33,
              position: 1,
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
        bobClient = await createClientWithFundedWallet(500000);

        bobIdentity = await bobClient.platform.identities.register(400000);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        expect(bobIdentity.constructor.name).to.be.equal('Identity');
      });

      it('should publish "Contacts" data contract', async () => {
        // 1. Create and broadcast data contract
        dataContract = await bobClient.platform
          .contracts.create(dataContractDocumentSchemas, bobIdentity);

        await bobClient.platform.contracts.publish(dataContract, bobIdentity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        bobClient.getApps().set('contacts', {
          contractId: dataContract.getId(),
          contract: dataContract,
        });

        // 2. Fetch and check data contract
        const fetchedDataContract = await bobClient.platform.contracts.get(
          dataContract.getId(),
        );

        expect(fetchedDataContract.toObject()).to.be.deep.equal(dataContract.toObject());
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

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        // 2. Fetch and compare profiles
        const [fetchedProfile] = await bobClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', profile.getId()]] },
        );

        expect(fetchedProfile.toObject()).to.be.deep.equal(profile.toObject());
      });

      it('should add encryption and decryption keys to the identity', async () => {
        const account = await bobClient.getWalletAccount();

        const numKeys = bobIdentity.getPublicKeys().length;
        const identityIndex = await account.getUnusedIdentityIndex();

        const { privateKey: encryptionPrivateKey } = account
          .identities
          .getIdentityHDKeyByIndex(identityIndex, 1);

        const { privateKey: decryptionPrivateKey } = account
          .identities
          .getIdentityHDKeyByIndex(identityIndex, 2);

        const encryptionPublicKey = new IdentityPublicKeyWithWitness(1);
        encryptionPublicKey.setId(numKeys + 1);
        encryptionPublicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);
        encryptionPublicKey.setPurpose(IdentityPublicKey.PURPOSES.ENCRYPTION);
        encryptionPublicKey.setContractBounds(dataContract.getId(), 'contact');
        encryptionPublicKey.setData(encryptionPrivateKey.toPublicKey().toBuffer());

        const decryptionPublicKey = new IdentityPublicKeyWithWitness(1);
        decryptionPublicKey.setId(numKeys + 2);
        decryptionPublicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);
        decryptionPublicKey.setPurpose(IdentityPublicKey.PURPOSES.DECRYPTION);
        decryptionPublicKey.setContractBounds(dataContract.getId(), 'contact');
        decryptionPublicKey.setData(decryptionPrivateKey.toPublicKey().toBuffer());

        const update = {
          add: [encryptionPublicKey, decryptionPublicKey],
        };

        await bobClient.platform.identities.update(
          bobIdentity,
          update,
          {
            [encryptionPublicKey.getId()]: encryptionPrivateKey,
            [decryptionPublicKey.getId()]: decryptionPrivateKey,
          },
        );

        await waitForSTPropagated();

        const { identitiesKeys } = await bobClient.getDAPIClient().platform
          .getIdentitiesContractKeys(
            [bobIdentity.getId()],
            dataContract.getId(),
            [IdentityPublicKey.PURPOSES.ENCRYPTION, IdentityPublicKey.PURPOSES.DECRYPTION],
            'contact',
          );

        const bobKeys = identitiesKeys[bobIdentity.getId().toString()];
        expect(bobKeys).to.exist();
        expect(bobKeys[IdentityPublicKey.PURPOSES.ENCRYPTION]).to.have.length(1);
        expect(bobKeys[IdentityPublicKey.PURPOSES.DECRYPTION]).to.have.length(1);
      });
    });

    describe('Alice', () => {
      it('should create user wallet and identity', async () => {
        // Create Alice wallet
        aliceClient = await createClientWithFundedWallet(500000);

        aliceClient.getApps().set('contacts', {
          contractId: dataContract.getId(),
          contract: dataContract,
        });

        aliceIdentity = await aliceClient.platform.identities.register(300000);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        expect(aliceIdentity.constructor.name).to.be.equal('Identity');
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

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        // 2. Fetch and compare profile
        const [fetchedProfile] = await aliceClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', aliceProfile.getId()]] },
        );

        expect(fetchedProfile.toObject()).to.be.deep.equal(aliceProfile.toObject());
      });

      it('should be able to update her profile', async () => {
        // 1. Update profile document
        aliceProfile.set('avatarUrl', 'http://test.com/alice2.jpg');

        // 2. Broadcast change
        await aliceClient.platform.documents.broadcast({
          replace: [aliceProfile],
        }, aliceIdentity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        // 3. Fetch and compare profile
        const [fetchedProfile] = await aliceClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', aliceProfile.getId()]] },
        );

        const fetchedProfileObject = fetchedProfile.toObject();
        delete fetchedProfileObject.$updatedAt;

        const aliceObject = aliceProfile.toObject();
        delete aliceObject.$updatedAt;

        expect(fetchedProfileObject).to.be.deep.equal({
          ...aliceObject,
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

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        // 2. Fetch and compare contacts
        const [fetchedContactRequest] = await bobClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', bobContactRequest.getId()]] },
        );

        expect(fetchedContactRequest.toObject()).to.be.deep.equal(bobContactRequest.toObject());
      });
    });

    describe('Alice', () => {
      it('should be able to approve contact request', async () => {
        // 1. Create and broadcast contact approval document
        aliceContactAcceptance = await aliceClient.platform.documents.create('contacts.contact', aliceIdentity, {
          toUserId: bobIdentity.getId(),
          publicKey: aliceIdentity.getPublicKeyById(0).getData(),
        });

        await aliceClient.platform.documents.broadcast({
          create: [aliceContactAcceptance],
        }, aliceIdentity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        // 2. Fetch and compare contacts
        const [fetchedAliceContactAcceptance] = await aliceClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', aliceContactAcceptance.getId()]] },
        );

        expect(fetchedAliceContactAcceptance.toObject()).to.be.deep.equal(
          aliceContactAcceptance.toObject(),
        );
      });

      it('should be able to remove contact approval', async () => {
        // 1. Broadcast document deletion
        await aliceClient.platform.documents.broadcast({
          delete: [aliceContactAcceptance],
        }, aliceIdentity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

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
