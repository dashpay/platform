const DAPIClient = require('@dashevo/dapi-client');
const DashPlatformProtocol = require('@dashevo/dpp');
const Identity = require('@dashevo/dpp/lib/identity/Identity');

const { PrivateKey } = require('@dashevo/dashcore-lib');

const createIdentity = require('../../lib/test/createIdentity');
const throwGrpcErrorWithMetadata = require('../../lib/test/throwGrpcErrorWithMetadata');

describe('Contacts', function contacts() {
  this.timeout(150000);
  this.retries(3);

  let dpp;
  let dataContract;

  let dapiClient;

  let bobIdentity;
  let bobPrivateKey;
  let bobContactRequest;
  let aliceIdentity;
  let alicePrivateKey;
  let aliceProfile;
  let aliceContactAcceptance;

  let dataContractDocumentSchemas;

  let stateRepository;

  before(() => {
    stateRepository = {
      dataContract: null,
      fetchDataContract() {
        return this.dataContract;
      },
    };

    dpp = new DashPlatformProtocol({
      stateRepository,
    });

    const seeds = process.env.DAPI_SEED
      .split(',')
      .map(seed => ({ service: `${seed}` }));

    dapiClient = new DAPIClient({
      seeds,
      timeout: 30000,
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

  describe('Bob', () => {
    it('should create user identity', async () => {
      bobPrivateKey = new PrivateKey();

      bobIdentity = await createIdentity(
        dpp,
        dapiClient,
        bobPrivateKey,
      );

      expect(bobIdentity).to.be.instanceOf(Identity);
    });

    it('should publish "Contacts" data contract', async () => {
      // 1. Create Data Contract
      dataContract = dpp.dataContract.create(
        bobIdentity.getId(),
        dataContractDocumentSchemas,
      );

      const result = await dpp.dataContract.validate(dataContract);
      expect(result.isValid(), 'Contract must be valid').to.be.true();

      stateRepository.dataContract = dataContract;

      // 3. Create State Transition
      const stateTransition = dpp.dataContract.createStateTransition(dataContract);

      stateTransition.sign(
        bobIdentity.getPublicKeyById(1),
        bobPrivateKey,
      );

      // 4. Send State Transition
      try {
        await dapiClient.applyStateTransition(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 5. Fetch Data Contract
      const actualContractSerialized = await dapiClient.getDataContract(dataContract.getId());

      const actualDataContract = await dpp.dataContract.createFromSerialized(
        actualContractSerialized,
      );

      expect(actualDataContract.toJSON()).to.be.deep.equal(dataContract.toJSON());
    });

    it('should create profile in "Contacts" app', async () => {
      // 1. Create profile
      const profile = dpp.document.create(dataContract, bobIdentity.getId(), 'profile', {
        avatarUrl: 'http://test.com/bob.jpg',
        about: 'This is story about me',
      });

      const result = await dpp.document.validate(profile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition({
        create: [profile],
      });

      stateTransition.sign(
        bobIdentity.getPublicKeyById(1),
        bobPrivateKey,
      );

      // 3. Send State Transition
      try {
        await dapiClient.applyStateTransition(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch profiles
      const [actualProfileSerialized] = await dapiClient.getDocuments(
        dataContract.getId(),
        'profile',
        { where: [['$id', '==', profile.getId()]] },
      );

      const actualProfile = await dpp.document.createFromSerialized(
        actualProfileSerialized,
      );

      expect(actualProfile.toJSON()).to.be.deep.equal(profile.toJSON());
    });
  });

  describe('Alice', () => {
    it('should create user identity', async () => {
      alicePrivateKey = new PrivateKey();

      aliceIdentity = await createIdentity(
        dpp,
        dapiClient,
        alicePrivateKey,
      );

      expect(aliceIdentity).to.be.instanceOf(Identity);
    });

    it('should create profile in "Contacts" app', async () => {
      // 1. Create Profile
      aliceProfile = dpp.document.create(dataContract, aliceIdentity.getId(), 'profile', {
        avatarUrl: 'http://test.com/alice.jpg',
        about: 'I am Alice',
      });

      const result = await dpp.document.validate(aliceProfile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition({
        create: [aliceProfile],
      });

      stateTransition.sign(
        aliceIdentity.getPublicKeyById(1),
        alicePrivateKey,
      );

      // 3. Send State Transition
      try {
        await dapiClient.applyStateTransition(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch profile
      const [actualAliceProfileSerialized] = await dapiClient.getDocuments(
        dataContract.getId(),
        'profile',
        { where: [['$id', '==', aliceProfile.getId()]] },
      );

      const actualAliceProfile = await dpp.document.createFromSerialized(
        actualAliceProfileSerialized,
      );

      expect(actualAliceProfile.toJSON()).to.be.deep.equal(aliceProfile.toJSON());
    });

    it('should be able to update her profile', async () => {
      // 1. Update profile document
      aliceProfile.set('avatarUrl', 'http://test.com/alice2.jpg');

      const result = await dpp.document.validate(aliceProfile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition({
        replace: [aliceProfile],
      });

      stateTransition.sign(
        aliceIdentity.getPublicKeyById(1),
        alicePrivateKey,
      );

      // 3. Send State Transition
      try {
        await dapiClient.applyStateTransition(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch profile
      const [actualAliceProfileSerialized] = await dapiClient.getDocuments(
        dataContract.getId(),
        'profile',
        { where: [['$id', '==', aliceProfile.getId()]] },
      );

      const actualAliceProfile = await dpp.document.createFromSerialized(
        actualAliceProfileSerialized,
      );

      expect(actualAliceProfile.toJSON()).to.be.deep.equal(aliceProfile.toJSON());
    });
  });

  describe('Bob', () => {
    it('should be able to send contact request', async () => {
      // 1. Create contact document
      bobContactRequest = dpp.document.create(dataContract, bobIdentity.getId(), 'contact', {
        toUserId: aliceIdentity.getId(),
        publicKey: bobIdentity.getPublicKeyById(1).getData(),
      });

      const result = await dpp.document.validate(bobContactRequest);
      expect(result.isValid(), 'Contact request must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition({
        create: [bobContactRequest],
      });

      stateTransition.sign(
        bobIdentity.getPublicKeyById(1),
        bobPrivateKey,
      );

      // 3. Send State Transition
      try {
        await dapiClient.applyStateTransition(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch contacts
      const [actualBobContactRequestSerialized] = await dapiClient.getDocuments(
        dataContract.getId(),
        'contact',
        { where: [['$id', '==', bobContactRequest.getId()]] },
      );

      const actualBobContactRequest = await dpp.document.createFromSerialized(
        actualBobContactRequestSerialized,
      );

      expect(actualBobContactRequest.toJSON()).to.be.deep.equal(bobContactRequest.toJSON());
    });
  });

  describe('Alice', () => {
    it('should be able to approve contact request', async () => {
      // 1. Create approve contract
      aliceContactAcceptance = dpp.document.create(dataContract, aliceIdentity.getId(), 'contact', {
        toUserId: bobIdentity.getId(),
        publicKey: aliceIdentity.getPublicKeyById(1).getData(),
      });

      const result = await dpp.document.validate(aliceContactAcceptance);
      expect(result.isValid(), 'Contact acceptance must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition({
        create: [aliceContactAcceptance],
      });

      stateTransition.sign(
        aliceIdentity.getPublicKeyById(1),
        alicePrivateKey,
      );

      // 3. Send State Transition
      try {
        await dapiClient.applyStateTransition(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch contacts
      const [actualAliceContactAcceptanceSerialized] = await dapiClient.getDocuments(
        dataContract.getId(),
        'contact',
        { where: [['$id', '==', aliceContactAcceptance.getId()]] },
      );

      const actualAliceContactAcceptance = await dpp.document.createFromSerialized(
        actualAliceContactAcceptanceSerialized,
      );

      expect(actualAliceContactAcceptance.toJSON()).to.be.deep.equal(
        aliceContactAcceptance.toJSON(),
      );
    });

    it('should be able to remove contact approvement', async () => {
      // 1. Create State Transition
      const stateTransition = dpp.document.createStateTransition({
        delete: [aliceContactAcceptance],
      });

      stateTransition.sign(
        aliceIdentity.getPublicKeyById(1),
        alicePrivateKey,
      );

      // 3. Send State Transition
      try {
        await dapiClient.applyStateTransition(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch contacts
      const [actualAliceContactAcceptance] = await dapiClient.getDocuments(
        dataContract.getId(),
        'contact',
        { where: [['$id', '==', aliceContactAcceptance.getId()]] },
      );

      expect(actualAliceContactAcceptance).to.not.exist();
    });
  });
});
