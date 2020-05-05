const {
  PrivateKey,
} = require('@dashevo/dashcore-lib');

const Dash = require('dash');
const DAPIClient = require('@dashevo/dapi-client');

const Identity = require('@dashevo/dpp/lib/identity/Identity');

const fundAddress = require('../../lib/test/fundAddress');

describe('e2e', () => {
  describe('Contacts', function contacts() {
    this.timeout(950000);

    let dataContract;

    let seeds;
    let dapiClient;
    let faucetPrivateKey;
    let faucetAddress;

    let bobDashClient;
    let aliceDashClient;

    let bobIdentity;
    let bobContactRequest;
    let aliceIdentity;
    let aliceProfile;
    let aliceContactAcceptance;

    let dataContractDocumentSchemas;

    before(() => {
      seeds = process.env.DAPI_SEED
        .split(',')
        .map((seed) => ({ service: `${seed}` }));

      // Prepare to fund Bob and Alice wallets
      faucetPrivateKey = PrivateKey.fromString(process.env.FAUCET_PRIVATE_KEY);
      faucetAddress = faucetPrivateKey
        .toAddress(process.env.NETWORK)
        .toString();

      dapiClient = new DAPIClient({
        seeds,
        timeout: 15000,
        retries: 10,
      });

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
              type: 'string',
            },
            publicKey: {
              type: 'string',
            },
          },
          required: ['toUserId', 'publicKey'],
          additionalProperties: false,
        },
      };
    });

    after(async () => {
      await bobDashClient.disconnect();
      await aliceDashClient.disconnect();
    });

    describe('Bob', () => {
      before(async () => {
        // Create Bob wallet
        bobDashClient = new Dash.Client({
          seeds,
          wallet: {
            transporter: {
              seeds,
              timeout: 15000,
              retries: 10,
              type: 'dapi',
            },
          },
          network: process.env.NETWORK,
        });

        await bobDashClient.isReady();

        // Send some Dash to Bob
        await fundAddress(
          dapiClient, faucetAddress, faucetPrivateKey,
          bobDashClient.account.getAddress().address,
          20000,
        );
      });

      it('should create user wallet and identity', async () => {
        bobIdentity = await bobDashClient.platform.identities.register();

        expect(bobIdentity).to.be.instanceOf(Identity);
      });

      it('should publish "Contacts" data contract', async () => {
        // 1. Create and broadcast data contract
        dataContract = await bobDashClient.platform.contracts.create(
          dataContractDocumentSchemas, bobIdentity,
        );

        await bobDashClient.platform.contracts.broadcast(dataContract, bobIdentity);

        bobDashClient.apps.contacts = {
          contractId: dataContract.getId(),
          contract: dataContract,
        };

        // 2. Fetch and check data contract
        const fetchedDataContract = await bobDashClient.platform.contracts.get(
          dataContract.getId(),
        );

        expect(fetchedDataContract.toJSON()).to.be.deep.equal(dataContract.toJSON());
      });

      it('should create profile in "Contacts" app', async () => {
        // 1. Create and broadcast profile
        const profile = await bobDashClient.platform.documents.create('contacts.profile', bobIdentity, {
          avatarUrl: 'http://test.com/bob.jpg',
          about: 'This is story about me',
        });

        await bobDashClient.platform.documents.broadcast({
          create: [profile],
        }, bobIdentity);

        // 2. Fetch and compare profiles
        const [fetchedProfile] = await bobDashClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', profile.getId()]] },
        );

        expect(fetchedProfile.toJSON()).to.be.deep.equal(profile.toJSON());
      });
    });

    describe('Alice', () => {
      before(async () => {
        // Create Alice wallet
        aliceDashClient = new Dash.Client({
          seeds,
          wallet: {
            transporter: {
              seeds,
              timeout: 15000,
              type: 'dapi',
            },
          },
          network: process.env.NETWORK,
        });

        await aliceDashClient.isReady();

        // Update contacts app
        aliceDashClient.apps.contacts = {
          contractId: dataContract.getId(),
          contract: dataContract,
        };

        // Send some Dash to Alice
        await fundAddress(
          dapiClient, faucetAddress, faucetPrivateKey,
          aliceDashClient.account.getAddress().address,
          20000,
        );
      });

      it('should create user wallet and identity', async () => {
        aliceIdentity = await aliceDashClient.platform.identities.register();

        expect(aliceIdentity).to.be.instanceOf(Identity);
      });

      it('should create profile in "Contacts" app', async () => {
        // 1. Create and broadcast profile
        aliceProfile = await aliceDashClient.platform.documents.create('contacts.profile', aliceIdentity, {
          avatarUrl: 'http://test.com/alice.jpg',
          about: 'I am Alice',
        });

        await aliceDashClient.platform.documents.broadcast({
          create: [aliceProfile],
        }, aliceIdentity);

        // 2. Fetch and compare profile
        const [fetchedProfile] = await aliceDashClient.platform.documents.get(
          'contacts.profile',
          { where: [['$id', '==', aliceProfile.getId()]] },
        );

        expect(fetchedProfile.toJSON()).to.be.deep.equal(aliceProfile.toJSON());
      });

      it('should be able to update her profile', async () => {
        // 1. Update profile document
        aliceProfile.set('avatarUrl', 'http://test.com/alice2.jpg');

        // 2. Broadcast change
        await aliceDashClient.platform.documents.broadcast({
          replace: [aliceProfile],
        }, aliceIdentity);

        // 3. Fetch and compare profile
        const [fetchedProfile] = await aliceDashClient.platform.documents.get(
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
        bobContactRequest = await bobDashClient.platform.documents.create('contacts.contact', bobIdentity, {
          toUserId: aliceIdentity.getId(),
          publicKey: bobIdentity.getPublicKeyById(0).getData(),
        });

        await bobDashClient.platform.documents.broadcast({
          create: [bobContactRequest],
        }, bobIdentity);

        // 2. Fetch and compare contacts
        const [fetchedContactRequest] = await bobDashClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', bobContactRequest.getId()]] },
        );

        expect(fetchedContactRequest.toJSON()).to.be.deep.equal(bobContactRequest.toJSON());
      });
    });

    describe('Alice', () => {
      it('should be able to approve contact request', async () => {
        // 1. Create and broadcast contact approval document
        aliceContactAcceptance = await aliceDashClient.platform.documents.create(
          'contacts.contact', aliceIdentity, {
            toUserId: bobIdentity.getId(),
            publicKey: aliceIdentity.getPublicKeyById(0).getData(),
          },
        );

        await aliceDashClient.platform.documents.broadcast({
          create: [aliceContactAcceptance],
        }, aliceIdentity);

        // 2. Fetch and compare contacts
        const [fetchedAliceContactAcceptance] = await aliceDashClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', aliceContactAcceptance.getId()]] },
        );

        expect(fetchedAliceContactAcceptance.toJSON()).to.be.deep.equal(
          aliceContactAcceptance.toJSON(),
        );
      });

      it('should be able to remove contact approval', async () => {
        // 1. Broadcast document deletion
        await aliceDashClient.platform.documents.broadcast({
          delete: [aliceContactAcceptance],
        }, aliceIdentity);

        // 2. Fetch contact documents and check it does not exists
        const [fetchedAliceContactAcceptance] = await aliceDashClient.platform.documents.get(
          'contacts.contact',
          { where: [['$id', '==', aliceContactAcceptance.getId()]] },
        );

        expect(fetchedAliceContactAcceptance).to.not.exist();
      });
    });
  });
});
