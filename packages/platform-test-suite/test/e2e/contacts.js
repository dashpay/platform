const DAPIClient = require('@dashevo/dapi-client');

const {
  Transaction,
  PrivateKey,
  PublicKey,
  Address,
} = require('@dashevo/dashcore-lib');

const Schema = require('@dashevo/dash-schema/dash-schema-lib');
const DashPay = require('@dashevo/dash-schema/dash-core-daps');

const doubleSha256 = require('../../lib/doubleSha256');
const wait = require('../../lib/wait');

describe('Contacts app', () => {
  const timeout = 1000;
  const attempts = 400;
  const testTimeout = 500000;

  let dapiClient;
  let dapId;
  let dapSchema;
  let dapContract;

  let faucetPrivateKey;
  let faucetAddress;

  let bobPrivateKey;
  let bobUserName;
  let bobRegTxId;
  let alicePrivateKey;
  let aliceUserName;
  let aliceRegTxId;

  let bobPreviousST;
  let alicePreviousST;

  before(() => {
    const seeds = process.env.DAPI_CLIENT_SEEDS
      .split(',')
      .map(ip => ({ ip }));

    dapiClient = new DAPIClient({
      seeds,
      port: process.env.DAPI_CLIENT_PORT,
    });

    faucetPrivateKey = new PrivateKey(process.env.FAUCET_PRIVATE_KEY);
    const faucetPublicKey = PublicKey.fromPrivateKey(faucetPrivateKey);
    faucetAddress = Address
      .fromPublicKey(faucetPublicKey, process.env.NETWORK === 'devnet' ? 'testnet' : process.env.NETWORK)
      .toString();

    bobUserName = Math.random().toString(36).substring(7);
    aliceUserName = Math.random().toString(36).substring(7);
    dapSchema = Object.assign({}, DashPay);
    dapSchema.title = `TestContacts_${bobUserName}`;

    dapContract = Schema.create.dapcontract(dapSchema);
    dapId = doubleSha256(Schema.serialize.encode(dapContract.dapcontract));
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
        .from(inputs.slice(-1)[0])
        .addFundingOutput(10000)
        .change(faucetAddress)
        .sign(faucetPrivateKey);

      ({ txid: bobRegTxId } = await dapiClient.sendRawTransaction(transaction.serialize()));

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
      let { stpacket: stPacket } = Schema.create.stpacket();
      stPacket = Object.assign(stPacket, dapContract);

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(bobRegTxId)
        .setHashPrevSubTx(bobPreviousST)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(bobPrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      bobPreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch DAP Contract
      let dapContractFromDAPI;
      for (let i = 0; i <= attempts; i++) {
        try {
          // waiting for Contacts to be added
          dapContractFromDAPI = await dapiClient.fetchDapContract(dapId);
          break;
        } catch (e) {
          await wait(timeout);
        }
      }

      expect(dapContractFromDAPI).to.have.property('dapname');
      expect(dapContractFromDAPI.dapname).to.be.equal(dapSchema.title);
    });

    it('should create profile in "Contacts" app', async function it() {
      this.timeout(testTimeout);

      const userRequest = Schema.create.dapobject('user');
      userRequest.aboutme = 'This is story about me';
      userRequest.avatar = 'My avatar here';
      userRequest.act = 0;

      // 1. Create ST profile packet
      const { stpacket: stPacket } = Schema.create.stpacket();
      stPacket.dapobjects = [userRequest];
      stPacket.dapid = dapId;

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(bobRegTxId)
        .setHashPrevSubTx(bobPreviousST)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(bobPrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      bobPreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch users
      let dapUsers;
      for (let i = 0; i <= attempts; i++) {
        dapUsers = await dapiClient.fetchDapObjects(dapId, 'user', {});
        // waiting for Bob's profile to be added
        if (dapUsers.length > 0) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(dapUsers).to.have.lengthOf(1);
      expect(dapUsers[0]).to.be.deep.equal(
        {
          act: 0,
          idx: 0,
          rev: 0,
          pver: null,
          avatar: 'My avatar here',
          aboutme: 'This is story about me',
          objtype: 'user',
        },
      );
    });
  });

  describe('Alice', () => {
    it('should register blockchain user', async function it() {
      this.timeout(50000);

      alicePrivateKey = new PrivateKey();
      const validPayload = new Transaction.Payload.SubTxRegisterPayload()
        .setUserName(aliceUserName)
        .setPubKeyIdFromPrivateKey(alicePrivateKey).sign(alicePrivateKey);

      const inputs = await dapiClient.getUTXO(faucetAddress);

      const transaction = Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_REGISTER)
        .setExtraPayload(validPayload)
        .from(inputs.slice(-1)[0])
        .addFundingOutput(10000)
        .change(faucetAddress)
        .sign(faucetPrivateKey);

      ({ txid: aliceRegTxId } = await dapiClient.sendRawTransaction(transaction.serialize()));

      alicePreviousST = aliceRegTxId;

      await dapiClient.generate(1);
      await wait(5000);

      const userByName = await dapiClient.getUserByName(aliceUserName);

      expect(userByName.uname).to.be.equal(aliceUserName);
    });

    it('should create profile in "Contacts" app', async function it() {
      this.timeout(testTimeout);

      const userRequest = Schema.create.dapobject('user');
      userRequest.aboutme = 'I am Alice';
      userRequest.avatar = 'Alice\'s avatar here';
      userRequest.act = 0;

      // 1. Create ST user packet
      const { stpacket: stPacket } = Schema.create.stpacket();
      stPacket.dapobjects = [userRequest];
      stPacket.dapid = dapId;

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch users
      let dapUsers;
      for (let i = 0; i <= attempts; i++) {
        dapUsers = await dapiClient.fetchDapObjects(dapId, 'user', {});
        // waiting for Alice's profile to be added
        if (dapUsers.length > 1) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(dapUsers).to.have.lengthOf(2);
      expect(dapUsers[1]).to.be.deep.equal(
        {
          act: 0,
          idx: 0,
          rev: 0,
          pver: null,
          avatar: 'Alice\'s avatar here',
          aboutme: 'I am Alice',
          objtype: 'user',
        },
      );
    });

    it('should be able to update her profile', async function it() {
      this.timeout(testTimeout);

      const userRequest = Schema.create.dapobject('user');
      userRequest.aboutme = 'I am Alice2';
      userRequest.avatar = 'Alice\'s avatar here2';

      // 1. Create ST update profile packet
      const { stpacket: stPacket } = Schema.create.stpacket();
      stPacket.dapobjects = [userRequest];
      stPacket.dapid = dapId;

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch users
      let dapUsers;
      for (let i = 0; i <= attempts; i++) {
        dapUsers = await dapiClient.fetchDapObjects(dapId, 'user', {});
        // waiting for Alice's profile modified
        if (dapUsers.length === 2 && dapUsers[1].act === 1) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(dapUsers).to.have.lengthOf(2);
      expect(dapUsers[1]).to.be.deep.equal(
        {
          act: 1,
          idx: 0,
          rev: 0,
          pver: null,
          avatar: 'Alice\'s avatar here2',
          aboutme: 'I am Alice2',
          objtype: 'user',
        },
      );
    });
  });

  describe('Bob', () => {
    it('should be able to send contact request', async function it() {
      this.timeout(testTimeout);

      const bobContactRequest = Schema.create.dapobject('contact');
      bobContactRequest.hdextpubkey = bobPrivateKey.toPublicKey().toString('hex');
      bobContactRequest.relation = aliceRegTxId;
      bobContactRequest.act = 0;

      // 1. Create ST contact request packet
      const { stpacket: stPacket } = Schema.create.stpacket();
      stPacket.dapobjects = [bobContactRequest];
      stPacket.dapid = dapId;

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(bobRegTxId)
        .setHashPrevSubTx(bobPreviousST)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(bobPrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      bobPreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch contacts
      let dapContacts;
      for (let i = 0; i <= attempts; i++) {
        dapContacts = await dapiClient.fetchDapObjects(dapId, 'contact', {});
        // waiting for Bob's contact request to be added
        if (dapContacts.length > 0) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(dapContacts).to.have.lengthOf(1);
      expect(dapContacts[0]).to.be.deep.equal(
        {
          act: 0,
          idx: 0,
          rev: 0,
          pver: null,
          objtype: 'contact',
          relation: aliceRegTxId,
          hdextpubkey: bobContactRequest.hdextpubkey,
        },
      );
    });
  });

  describe('Alice', () => {
    it('should be able to approve contact request', async function it() {
      this.timeout(testTimeout);

      const contactAcceptance = Schema.create.dapobject('contact');
      contactAcceptance.hdextpubkey = alicePrivateKey.toPublicKey().toString('hex');
      contactAcceptance.relation = bobRegTxId;

      // 1. Create ST approve contact packet
      const { stpacket: stPacket } = Schema.create.stpacket();
      stPacket.dapobjects = [contactAcceptance];
      stPacket.dapid = dapId;

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch contacts
      let dapContacts;
      for (let i = 0; i <= attempts; i++) {
        dapContacts = await dapiClient.fetchDapObjects(dapId, 'contact', {});
        // waiting for Bob's contact to be approved from Alice
        if (dapContacts.length > 1) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(dapContacts).to.have.lengthOf(2);
      expect(dapContacts[1]).to.be.deep.equal(
        {
          act: 1,
          idx: 0,
          rev: 0,
          pver: null,
          objtype: 'contact',
          relation: bobRegTxId,
          hdextpubkey: contactAcceptance.hdextpubkey,
        },
      );
    });

    it('should be able to remove contact approvement', async function it() {
      this.timeout(testTimeout);

      const contactDeleteRequest = Schema.create.dapobject('contact');
      contactDeleteRequest.hdextpubkey = alicePrivateKey.toPublicKey().toString('hex');
      contactDeleteRequest.relation = bobRegTxId;
      contactDeleteRequest.act = 2;

      // 1. Create ST contact delete packet
      const { stpacket: stPacket } = Schema.create.stpacket();
      stPacket.dapobjects = [contactDeleteRequest];
      stPacket.dapid = dapId;

      // 2. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(aliceRegTxId)
        .setHashPrevSubTx(alicePreviousST)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(alicePrivateKey);

      const transitionHash = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transitionHash).to.be.a('string');
      expect(transitionHash).to.be.not.empty();

      alicePreviousST = transitionHash;

      // 3. Mine block with ST
      await dapiClient.generate(1);

      // 4. Fetch contacts
      let aliceContact;
      for (let i = 0; i <= attempts; i++) {
        // waiting for Bob's contact to be deleted from Alice
        aliceContact = await dapiClient.fetchDapObjects(dapId, 'contact', {});
        if (aliceContact.length === 1) {
          break;
        } else {
          await wait(timeout);
        }
      }

      expect(aliceContact).to.have.lengthOf(1);
      expect(aliceContact[0]).to.be.deep.equal(
        {
          act: 0,
          idx: 0,
          rev: 0,
          pver: null,
          objtype: 'contact',
          relation: aliceRegTxId,
          hdextpubkey: bobPrivateKey.toPublicKey().toString('hex'),
        },
      );
    });
  });
});
