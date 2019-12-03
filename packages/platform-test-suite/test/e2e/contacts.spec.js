const DAPIClient = require('@dashevo/dapi-client');
const DashPlatformProtocol = require('@dashevo/dpp');
const Document = require('@dashevo/dpp/lib/document/Document');

const {
  Transaction,
  PrivateKey,
  PublicKey,
  Address,
} = require('@dashevo/dashcore-lib');

const throwGrpcErrorWithMetadata = require('../../lib/test/throwGrpcErrorWithMetadata');

const wait = require('../../lib/wait');

describe('Contacts app', () => {
  const testTimeout = 600000;

  let dpp;
  let dataContract;

  let dapiClient;

  let faucetPrivateKey;
  let faucetAddress;

  let bobPrivateKey;
  let bobUserName;
  let bobRegTxId;
  let bobContactRequest;
  let alicePrivateKey;
  let aliceUserName;
  let aliceRegTxId;
  let aliceProfile;
  let aliceContactAcceptance;

  let dataContractDocumentSchemas;

  let dataProvider;

  before(() => {
    dataProvider = {
      dataContract: null,
      fetchDataContract() {
        return dataContract;
      },
    };

    dpp = new DashPlatformProtocol({
      dataProvider,
    });

    const seeds = process.env.DAPI_CLIENT_SEEDS
      .split(',')
      .map(ip => ({ service: `${ip}:${process.env.DAPI_CLIENT_PORT}` }));

    dapiClient = new DAPIClient({
      seeds,
      timeout: 30000,
    });

    faucetPrivateKey = new PrivateKey(process.env.FAUCET_PRIVATE_KEY);
    const faucetPublicKey = PublicKey.fromPrivateKey(faucetPrivateKey);
    faucetAddress = Address
      .fromPublicKey(faucetPublicKey, process.env.NETWORK === 'devnet' ? 'testnet' : process.env.NETWORK)
      .toString();

    bobUserName = Math.random().toString(36).substring(7);
    aliceUserName = Math.random().toString(36).substring(7);

    dataContractDocumentSchemas = {
      profile: {
        indices: [
          { properties: [{ $userId: 'asc' }], unique: true },
        ],
        properties: {
          avatarUrl: {
            type: 'string',
            format: 'url',
          },
          about: {
            type: 'string',
          },
        },
        required: ['avatarUrl', 'about'],
        additionalProperties: false,
      },
      contact: {
        indices: [
          { properties: [{ $userId: 'asc' }, { toUserId: 'asc' }], unique: true },
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
    it('should register blockchain user', async function it() {
      this.timeout(50000);

      bobPrivateKey = new PrivateKey();
      const validPayload = new Transaction.Payload.SubTxRegisterPayload()
        .setUserName(bobUserName)
        .setPubKeyIdFromPrivateKey(bobPrivateKey)
        .sign(bobPrivateKey);

      const { items: inputs } = await dapiClient.getUTXO(faucetAddress);

      expect(inputs).to.be.an('array').and.not.empty();

      const transaction = Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER)
        .setExtraPayload(validPayload)
        .from(inputs.slice(-1)[0])
        .addFundingOutput(10000)
        .change(faucetAddress)
        .sign(faucetPrivateKey);

      bobRegTxId = await dapiClient.sendRawTransaction(transaction.serialize());

      expect(bobRegTxId).to.be.a('string');

      await dapiClient.generate(1);
      await wait(5000);

      const userByName = await dapiClient.getUserByName(bobUserName);
      expect(userByName.uname).to.be.equal(bobUserName);
    });

    it('should publish "Contacts" contract', async function it() {
      this.timeout(testTimeout);

      // 1. Create Data Contract
      dataContract = dpp.dataContract.create(bobRegTxId, dataContractDocumentSchemas);

      const result = dpp.dataContract.validate(dataContract);
      expect(result.isValid(), 'Contract must be valid').to.be.true();

      dataProvider.dataContract = dataContract;

      // 2. Create State Transition
      const stateTransition = dpp.dataContract.createStateTransition(dataContract);

      // 3. Send State Transition
      try {
        await dapiClient.updateState(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch Data Contract
      const actualContract = await dapiClient.fetchContract(dataContract.getId());

      expect(actualContract).to.be.deep.equal(dataContract.toJSON());
    });

    it('should create profile in "Contacts" app', async function it() {
      this.timeout(testTimeout);

      // 1. Create profile
      const profile = dpp.document.create(dataContract, bobRegTxId, 'profile', {
        avatarUrl: 'http://test.com/bob.jpg',
        about: 'This is story about me',
      });

      const result = await dpp.document.validate(profile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition([profile]);

      // 3. Send State Transition
      try {
        await dapiClient.updateState(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch profiles
      const [actualProfile] = await dapiClient.fetchDocuments(
        dataContract.getId(),
        'profile',
        { where: [['$id', '==', profile.getId()]] },
      );

      expect(actualProfile).to.be.deep.equal(profile.toJSON());
    });
  });

  describe('Alice', () => {
    it('should register blockchain user', async function it() {
      this.timeout(50000);

      alicePrivateKey = new PrivateKey();
      const validPayload = new Transaction.Payload.SubTxRegisterPayload()
        .setUserName(aliceUserName)
        .setPubKeyIdFromPrivateKey(alicePrivateKey).sign(alicePrivateKey);

      const { items: inputs } = await dapiClient.getUTXO(faucetAddress);

      expect(inputs).to.be.an('array').and.not.empty();

      const transaction = Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER)
        .setExtraPayload(validPayload)
        .from(inputs.slice(-1)[0])
        .addFundingOutput(10000)
        .change(faucetAddress)
        .sign(faucetPrivateKey);

      aliceRegTxId = await dapiClient.sendRawTransaction(transaction.serialize());

      await dapiClient.generate(1);
      await wait(5000);

      const userByName = await dapiClient.getUserByName(aliceUserName);

      expect(userByName.uname).to.be.equal(aliceUserName);
    });

    it('should create profile in "Contacts" app', async function it() {
      this.timeout(testTimeout);

      // 1. Create Profile
      aliceProfile = dpp.document.create(dataContract, aliceRegTxId, 'profile', {
        avatarUrl: 'http://test.com/alice.jpg',
        about: 'I am Alice',
      });

      const result = await dpp.document.validate(aliceProfile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition([aliceProfile]);

      // 3. Send State Transition
      try {
        await dapiClient.updateState(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch profile
      const [actualAliceProfile] = await dapiClient.fetchDocuments(
        dataContract.getId(),
        'profile',
        { where: [['$id', '==', aliceProfile.getId()]] },
      );

      expect(actualAliceProfile).to.be.deep.equal(aliceProfile.toJSON());
    });

    it('should be able to update her profile', async function it() {
      this.timeout(testTimeout);

      // 1. Update profile document
      aliceProfile.setAction(Document.ACTIONS.REPLACE);
      aliceProfile.setRevision(2);
      aliceProfile.set('avatarUrl', 'http://test.com/alice2.jpg');

      const result = await dpp.document.validate(aliceProfile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition([aliceProfile]);

      // 3. Send State Transition
      try {
        await dapiClient.updateState(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch profile
      const [actualAliceProfile] = await dapiClient.fetchDocuments(
        dataContract.getId(),
        'profile',
        { where: [['$id', '==', aliceProfile.getId()]] },
      );

      expect(actualAliceProfile).to.be.deep.equal(aliceProfile.toJSON());
    });
  });

  describe('Bob', () => {
    it('should be able to send contact request', async function it() {
      this.timeout(testTimeout);

      // 1. Create contact document
      bobContactRequest = dpp.document.create(dataContract, bobRegTxId, 'contact', {
        toUserId: aliceRegTxId,
        publicKey: bobPrivateKey.toPublicKey().toString('hex'),
      });

      const result = await dpp.document.validate(bobContactRequest);
      expect(result.isValid(), 'Contact request must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition([bobContactRequest]);

      // 3. Send State Transition
      try {
        await dapiClient.updateState(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch contacts
      const [actualBobContactRequest] = await dapiClient.fetchDocuments(
        dataContract.getId(),
        'contact',
        { where: [['$id', '==', bobContactRequest.getId()]] },
      );

      expect(actualBobContactRequest).to.be.deep.equal(bobContactRequest.toJSON());
    });
  });

  describe('Alice', () => {
    it('should be able to approve contact request', async function it() {
      this.timeout(testTimeout);

      // 1. Create approve contract
      aliceContactAcceptance = dpp.document.create(dataContract, aliceRegTxId, 'contact', {
        toUserId: bobRegTxId,
        publicKey: alicePrivateKey.toPublicKey().toString('hex'),
      });

      const result = await dpp.document.validate(aliceContactAcceptance);
      expect(result.isValid(), 'Contact acceptance must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition([aliceContactAcceptance]);

      // 3. Send State Transition
      try {
        await dapiClient.updateState(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch contacts
      const [actualAliceContactAcceptance] = await dapiClient.fetchDocuments(
        dataContract.getId(),
        'contact',
        { where: [['$id', '==', aliceContactAcceptance.getId()]] },
      );

      expect(actualAliceContactAcceptance).to.be.deep.equal(aliceContactAcceptance.toJSON());
    });

    it('should be able to remove contact approvement', async function it() {
      this.timeout(testTimeout);

      // 1. Remove contract approvement
      aliceContactAcceptance.setData({});
      aliceContactAcceptance.setAction(Document.ACTIONS.DELETE);
      aliceContactAcceptance.setRevision(2);

      const result = await dpp.document.validate(aliceContactAcceptance);
      expect(result.isValid(), 'Contact acceptance must be valid').to.be.true();

      // 2. Create State Transition
      const stateTransition = dpp.document.createStateTransition([aliceContactAcceptance]);

      // 3. Send State Transition
      try {
        await dapiClient.updateState(stateTransition);
      } catch (e) {
        throwGrpcErrorWithMetadata(e);
      }

      // 4. Fetch contacts
      const [actualAliceContactAcceptance] = await dapiClient.fetchDocuments(
        dataContract.getId(),
        'contact',
        { where: [['$id', '==', aliceContactAcceptance.getId()]] },
      );

      expect(actualAliceContactAcceptance).to.not.exist();
    });
  });
});
