require('../../bootstrap');

const path = require('path');
const dotenvSafe = require('dotenv-safe');

const sinon = require('sinon');

const { startDapi } = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('@dashevo/dpp');
const STPacketFactory = require('@dashevo/dpp/lib/stPacket/STPacketFactory');
const entropy = require('@dashevo/dpp/lib/util/entropy');
const Document = require('@dashevo/dpp/lib/document/Document');

const {
  Transaction,
  PrivateKey,
  PublicKey,
  Address,
} = require('@dashevo/dashcore-lib');
const DAPIClient = require('../../../src/index');
const MNDiscovery = require('../../../src/MNDiscovery/index');

const wait = require('../../utils/wait');

process.env.NODE_ENV = 'test';

dotenvSafe.config({
  sample: path.resolve(__dirname, '../.env'),
  path: path.resolve(__dirname, '../.env'),
});



describe('basic E2E tests', () => {
  let masterNode;

  const attempts = 60;

  let dpp;

  let dapiClient;

  let faucetPrivateKey;
  let faucetAddress;

  let bobPrivateKey;
  let bobUserName;
  let bobRegTxId;
  let alicePrivateKey;
  let aliceUserName;
  let aliceRegTxId;

  let aliceProfile;
  let bobContactRequest;
  let aliceContactAcceptance;

  let bobPreviousST;
  let alicePreviousST;

  before(async () => {

    dpp = new DashPlatformProtocol();
    const privKey = 'cVwyvFt95dzwEqYCLd8pv9CzktajP4tWH2w9RQNPeHYA7pH35wcJ';
    faucetPrivateKey = new PrivateKey(privKey);

    const faucetPublicKey = PublicKey.fromPrivateKey(faucetPrivateKey);

    faucetAddress = Address
      .fromPublicKey(faucetPublicKey, 'testnet')
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

    sinon.stub(MNDiscovery.prototype, 'getRandomMasternode')
      .returns(Promise.resolve({ service: '127.0.0.1' }));

    [masterNode] = await startDapi.many(1);

    const seeds = [{ service: masterNode.dapiCore.container.getIp() }];
    await masterNode.dashCore.getApi().generate(1500);

    dapiClient = new DAPIClient({
      seeds,
      port: masterNode.dapiCore.options.getRpcPort(),
    });

    // dash-cli -regtest -rpcuser=dashrpc -rpcpassword=password -rpcport=21456 sendtoaddress ygPcCwVy7Fxg7ruxZzqVYdPLtvw7auHAFh 1

    await masterNode.dashCore.getApi().sendToAddress(faucetAddress, 100);
    await dapiClient.generate(20);
    await wait(10000);
  });

  after('cleanup lone services', async () => {
    const instances = [
      masterNode,
    ];

    await Promise.all(instances.filter(i => i)
      .map(i => i.remove()));

    MNDiscovery.prototype.getRandomMasternode.restore();
  });

  describe('Bob', () => {
    it('should register blockchain user', async function it() {
      this.timeout(50000);

      bobPrivateKey = new PrivateKey();
      const validPayload = new Transaction.Payload.SubTxRegisterPayload()
        .setUserName(bobUserName)
        .setPubKeyIdFromPrivateKey(bobPrivateKey).sign(bobPrivateKey);

      const inputs = await dapiClient.getUTXO(faucetAddress);

      const transaction = Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER)
        .setExtraPayload(validPayload)
        .from(inputs.items)
        .addFundingOutput(10000)
        .change(faucetAddress)
        .sign(faucetPrivateKey);

      bobRegTxId = await dapiClient.sendRawTransaction(transaction.serialize());

      bobPreviousST = bobRegTxId;

      await dapiClient.generate(1);
      await wait(5000);

      const userByName = await dapiClient.getUserByName(bobUserName);
      expect(userByName.uname).to.be.equal(bobUserName);
    });

    it('should publish "Contacts" contract', async () => {
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

      let contract;
      await wait(5000);
      for (let i = 0; i <= attempts; i++) {
        try {
          // waiting for Contacts to be added
          contract = await dapiClient.fetchContract(dpp.getContract().getId());
          break;
        } catch (e) {
          await dapiClient.generate(1);
        }
      }

      const expectedContract = JSON.parse(JSON.stringify(dpp.getContract()));
      delete expectedContract.definitions;
      delete expectedContract.schema;
      expectedContract.$schema = 'https://schema.dash.org/dpp-0-4-0/meta/contract';
      expect(contract).to.be.deep.equal(expectedContract);
    });

    it('should create profile in "Contacts" app', async () => {
      dpp.setUserId(bobRegTxId);

      const profile = dpp.document.create('profile', {
        avatarUrl: 'http://test.com/bob.jpg',
        about: 'This is story about me',
      });

      profile.removeMetadata();

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

      let profiles;
      for (let i = 0; i <= attempts; i++) {
        profiles = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'profile',
          {},
        );
        // waiting for Bob's profile to be added
        if (profiles.length > 0) {
          break;
        } else {
          await dapiClient.generate(1);
        }
      }
      expect(profiles).to.have.lengthOf(1);
      expect(profiles[0].$meta).to.be.deep.equal({"userId": bobRegTxId});

      delete profiles[0].$meta;
      expect(profiles[0]).to.be.deep.equal(profile.toJSON());
    });

  });

  describe('Alice', () => {
    it('should register blockchain user', async function it() {
      this.timeout(50000);

      await dapiClient.generate(20);

      alicePrivateKey = new PrivateKey();
      const validPayload = new Transaction.Payload.SubTxRegisterPayload()
        .setUserName(aliceUserName)
        .setPubKeyIdFromPrivateKey(alicePrivateKey).sign(alicePrivateKey);

      const inputs = await dapiClient.getUTXO(faucetAddress);
      expect(inputs.items).to.have.lengthOf(1);

      const transaction = Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER)
        .setExtraPayload(validPayload)
        .from(inputs.items)
        .addFundingOutput(10000)
        .change(faucetAddress)
        .sign(faucetPrivateKey);

      aliceRegTxId = await dapiClient.sendRawTransaction(transaction.serialize());

      alicePreviousST = aliceRegTxId;

      const userByName = await dapiClient.getUserByName(aliceUserName);
      expect(userByName.uname).to.be.equal(aliceUserName);
    });

    it('should create profile in "Contacts" app', async () => {
      dpp.setUserId(aliceRegTxId);

      aliceProfile = dpp.document.create('profile', {
        avatarUrl: 'http://test.com/alice.jpg',
        about: 'I am Alice',
      });

      aliceProfile.removeMetadata();

      // 1. Create ST user packet
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

      let profiles;
      for (let i = 0; i <= attempts; i++) {
        profiles = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'profile',
          {},
        );
        // waiting for Alice's profile to be added
        if (profiles.length > 1) {
          break;
        } else {
          await dapiClient.generate(1);
        }
      }

      expect(profiles).to.have.lengthOf(2);
      expect(profiles[1].$meta).to.be.deep.equal({"userId": aliceRegTxId});

      delete profiles[1].$meta;
      expect(profiles[1]).to.be.deep.equal(aliceProfile.toJSON());
    });

    it('should be able to update her profile', async () => {
      dpp.setUserId(aliceRegTxId);

      aliceProfile.setAction(Document.ACTIONS.UPDATE);
      aliceProfile.setRevision(aliceProfile.revision + 1);
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

      let profiles;
      for (let i = 0; i <= attempts; i++) {
        profiles = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'profile',
          {},
        );
        // waiting for Alice's profile modified
        if (profiles.length === 2 && profiles[1].act === 1) {
          break;
        } else {
          await dapiClient.generate(1);
        }
      }

      expect(profiles).to.have.lengthOf(2);
      expect(profiles[1].$meta).to.be.deep.equal({"userId": aliceRegTxId});

      delete profiles[1].$meta;
      expect(profiles[1]).to.be.deep.equal(aliceProfile.toJSON());
    });
  });

  describe('Bob', () => {
    it('should be able to send contact request', async () => {
      dpp.setUserId(bobRegTxId);

      bobContactRequest = dpp.document.create('contact', {
        toUserId: aliceRegTxId,
        publicKey: bobPrivateKey.toPublicKey().toString('hex'),
      });

      bobContactRequest.removeMetadata();

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

      let contacts;
      for (let i = 0; i <= attempts; i++) {
        contacts = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'contact',
          {},
        );
        // waiting for Bob's contact request to be added
        if (contacts.length > 0) {
          break;
        } else {
          await dapiClient.generate(1);
        }
      }

      expect(contacts).to.have.lengthOf(1);
      expect(contacts[0].$meta).to.be.deep.equal({"userId": bobRegTxId});

      delete contacts[0].$meta;
      expect(contacts[0]).to.be.deep.equal(bobContactRequest.toJSON());
    });
  });

  describe('Alice', () => {
    it('should be able to approve contact request', async () => {
      dpp.setUserId(aliceRegTxId);

      aliceContactAcceptance = dpp.document.create('contact', {
        toUserId: bobRegTxId,
        publicKey: alicePrivateKey.toPublicKey().toString('hex'),
      });

      aliceContactAcceptance.removeMetadata();

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

      let contacts;
      for (let i = 0; i <= attempts; i++) {
        contacts = await dapiClient.fetchDocuments(
          dpp.getContract().getId(),
          'contact',
          {},
        );
        // waiting for Bob's contact to be approved from Alice
        if (contacts.length > 1) {
          break;
        } else {
          await dapiClient.generate(1);
          await wait(1000);
        }
      }

      expect(contacts).to.have.lengthOf(2);
      expect(contacts[1].$meta).to.be.deep.equal({"userId": aliceRegTxId});

      delete contacts[1].$meta;
      expect(contacts[1]).to.be.deep.equal(aliceContactAcceptance.toJSON());
    });

    it('should be able to remove contact approvement', async () => {
      dpp.setUserId(aliceRegTxId);

      aliceContactAcceptance.setData({});
      aliceContactAcceptance.setAction(Document.ACTIONS.DELETE);
      aliceContactAcceptance.setRevision(aliceContactAcceptance.revision + 1);

      aliceContactAcceptance.removeMetadata();

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
          await wait(1000);
        }
      }

      expect(contacts).to.have.lengthOf(1);
      expect(contacts[0].$meta).to.be.deep.equal({"userId": bobRegTxId});

      delete contacts[0].$meta;
      expect(contacts[0]).to.be.deep.equal(bobContactRequest.toJSON());
    });
  });
});
