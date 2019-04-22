const DAPIClient = require('@dashevo/dapi-client');
const DashPlatformProtocol = require('@dashevo/dpp');
const Document = require('@dashevo/dpp/lib/document/Document');

const {
  Transaction,
  PrivateKey,
  PublicKey,
  Address,
} = require('@dashevo/dashcore-lib');

const wait = require('../../lib/wait');

describe('Contacts app', () => {
  const timeout = 1000;
  const attempts = 400;
  const testTimeout = 600000;

  let dpp;

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

  let bobPreviousST;
  let alicePreviousST;

  before(() => {
    dpp = new DashPlatformProtocol();

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

    const contractName = Math.random().toString(36).substring(7);
    const contract = dpp.contract.create(contractName, {
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
    });

    const result = dpp.contract.validate(contract);
    expect(result.isValid(), 'Contract must be valid').to.be.true();

    dpp.setContract(contract);
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

      bobPreviousST = bobRegTxId;

      await dapiClient.generate(1);
      await wait(5000);

      const userByName = await dapiClient.getUserByName(bobUserName);
      expect(userByName.uname).to.be.equal(bobUserName);
    });

    it('should publish "Contacts" contract', async function it() {
      this.timeout(testTimeout);

      // 1. Create ST packet
      const stPacket = dpp.packet.create(dpp.getContract());

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      transaction.extraPayload
        .setRegTxId(bobRegTxId)
        .setHashPrevSubTx(bobPreviousST)
        .setHashSTPacket(stPacket.hash())
        .setCreditFee(1000)
        .sign(bobPrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        stPacket.serialize().toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      bobPreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch DAP Contract
      let actualContract;
      for (let i = 0; i <= attempts; i++) {
        try {
          // waiting for Contacts to be added
          actualContract = await dapiClient.fetchContract(dpp.getContract().getId());
          if (actualContract) {
            break;
          }
        } catch (e) {
          await wait(timeout);
        }
      }

      expect(actualContract).to.be.deep.equal(dpp.getContract().toJSON());
    });

    it('should create profile in "Contacts" app', async function it() {
      this.timeout(testTimeout);

      dpp.setUserId(bobRegTxId);

      const profile = dpp.document.create('profile', {
        avatarUrl: 'http://test.com/bob.jpg',
        about: 'This is story about me',
      });

      const result = dpp.document.validate(profile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 1. Create ST profile packet
      const stPacket = dpp.packet.create([profile]);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      transaction.extraPayload
        .setRegTxId(bobRegTxId)
        .setHashPrevSubTx(bobPreviousST)
        .setHashSTPacket(stPacket.hash())
        .setCreditFee(1000)
        .sign(bobPrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        stPacket.serialize().toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      bobPreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch profiles
      let actualProfile;
      for (let i = 0; i <= attempts; i++) {
        [actualProfile] = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'profile',
          { where: { _id: profile.getId() } },
        );

        // waiting for Bob's profile to be added
        if (actualProfile) {
          break;
        } else {
          await wait(timeout);
        }
      }

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

      alicePreviousST = aliceRegTxId;

      await dapiClient.generate(1);
      await wait(5000);

      const userByName = await dapiClient.getUserByName(aliceUserName);

      expect(userByName.uname).to.be.equal(aliceUserName);
    });

    it('should create profile in "Contacts" app', async function it() {
      this.timeout(testTimeout);

      dpp.setUserId(aliceRegTxId);

      aliceProfile = dpp.document.create('profile', {
        avatarUrl: 'http://test.com/alice.jpg',
        about: 'I am Alice',
      });

      const result = dpp.document.validate(aliceProfile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 1. Create ST Packet
      const stPacket = dpp.packet.create([aliceProfile]);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacket.hash())
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        stPacket.serialize().toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch profiles
      let actualAliceProfile;
      for (let i = 0; i <= attempts; i++) {
        [actualAliceProfile] = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'profile',
          { where: { _id: aliceProfile.getId() } },
        );

        // waiting for Alice's profile to be added
        if (actualAliceProfile) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(actualAliceProfile).to.be.deep.equal(aliceProfile.toJSON());
    });

    it('should be able to update her profile', async function it() {
      this.timeout(testTimeout);

      dpp.setUserId(aliceRegTxId);

      aliceProfile.setAction(Document.ACTIONS.UPDATE);
      aliceProfile.setRevision(2);
      aliceProfile.set('avatarUrl', 'http://test.com/alice2.jpg');

      const result = dpp.document.validate(aliceProfile);
      expect(result.isValid(), 'Profile must be valid').to.be.true();

      // 1. Create ST update profile packet
      const stPacket = dpp.packet.create([aliceProfile]);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacket.hash())
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        stPacket.serialize().toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch profile
      let actualAliceProfile;
      for (let i = 0; i <= attempts; i++) {
        [actualAliceProfile] = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'profile',
          { where: { _id: aliceProfile.getId() } },
        );

        // waiting for Alice's profile modified
        if (actualAliceProfile && actualAliceProfile.$rev === 2) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(actualAliceProfile).to.be.deep.equal(aliceProfile.toJSON());
    });
  });

  describe('Bob', () => {
    it('should be able to send contact request', async function it() {
      this.timeout(testTimeout);

      dpp.setUserId(bobRegTxId);

      bobContactRequest = dpp.document.create('contact', {
        toUserId: aliceRegTxId,
        publicKey: bobPrivateKey.toPublicKey().toString('hex'),
      });

      const result = dpp.document.validate(bobContactRequest);
      expect(result.isValid(), 'Contact request must be valid').to.be.true();

      // 1. Create ST contact request packet
      const stPacket = dpp.packet.create([bobContactRequest]);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      transaction.extraPayload
        .setRegTxId(bobRegTxId)
        .setHashPrevSubTx(bobPreviousST)
        .setHashSTPacket(stPacket.hash())
        .setCreditFee(1000)
        .sign(bobPrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        stPacket.serialize().toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      bobPreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch contacts
      let actualBobContactRequest;
      for (let i = 0; i <= attempts; i++) {
        [actualBobContactRequest] = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'contact',
          { where: { _id: bobContactRequest.getId() } },
        );

        // waiting for Bob's contact request to be added
        if (actualBobContactRequest) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(actualBobContactRequest).to.be.deep.equal(bobContactRequest.toJSON());
    });
  });

  describe('Alice', () => {
    it('should be able to approve contact request', async function it() {
      this.timeout(testTimeout);

      dpp.setUserId(aliceRegTxId);

      aliceContactAcceptance = dpp.document.create('contact', {
        toUserId: bobRegTxId,
        publicKey: alicePrivateKey.toPublicKey().toString('hex'),
      });

      const result = dpp.document.validate(aliceContactAcceptance);
      expect(result.isValid(), 'Contact acceptance must be valid').to.be.true();

      // 1. Create ST approve contact packet
      const stPacket = dpp.packet.create([aliceContactAcceptance]);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacket.hash())
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        stPacket.serialize().toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch contacts
      let actualAliceContactAcceptance;
      for (let i = 0; i <= attempts; i++) {
        [actualAliceContactAcceptance] = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'contact',
          { where: { _id: aliceContactAcceptance.getId() } },
        );

        // waiting for Bob's contact to be approved from Alice
        if (actualAliceContactAcceptance) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(actualAliceContactAcceptance).to.be.deep.equal(aliceContactAcceptance.toJSON());
    });

    it('should be able to remove contact approvement', async function it() {
      this.timeout(testTimeout);

      dpp.setUserId(aliceRegTxId);

      aliceContactAcceptance.setData({});
      aliceContactAcceptance.setAction(Document.ACTIONS.DELETE);
      aliceContactAcceptance.setRevision(2);

      const result = dpp.document.validate(aliceContactAcceptance);
      expect(result.isValid(), 'Contact acceptance must be valid').to.be.true();

      // 1. Create ST contact delete packet
      const stPacket = dpp.packet.create([aliceContactAcceptance]);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacket.hash())
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        stPacket.serialize().toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch contacts
      let contacts;
      for (let i = 0; i <= attempts; i++) {
        // waiting for Bob's contact to be deleted from Alice
        contacts = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'contact',
          {},
        );

        if (contacts.length === 1) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(contacts).to.have.lengthOf(1);
      expect(contacts[0]).to.be.deep.equal(bobContactRequest.toJSON());
    });
  });
});
