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
  let dapiClient;

  let faucetPrivateKey;
  let faucetPublicKey;

  let bobPrivateKey;
  let bobUserName;
  let bobRegTxId;

  before(() => {
    const seeds = process.env.DAPI_CLIENT_SEEDS
      .split(',')
      .map(ip => ({ ip }));

    dapiClient = new DAPIClient({
      seeds,
      port: process.env.DAPI_CLIENT_PORT,
    });

    faucetPrivateKey = new PrivateKey(process.env.FAUCET_PRIVATE_KEY);
    faucetPublicKey = PublicKey.fromPrivateKey(faucetPrivateKey);
    bobUserName = Math.random().toString(36).substring(7);
  });

  describe('Bob', () => {
    it('should register blockchain user', async function it() {
      this.timeout(150000);

      const faucetAddress = Address
        .fromPublicKey(faucetPublicKey, process.env.NETWORK === 'devnet' ? 'testnet' : process.env.NETWORK)
        .toString();

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

      // await dapiClient.generate(1);
      await wait(5000);

      const userByName = await dapiClient.getUserByName(bobUserName);
      expect(userByName.uname).to.be.equal(bobUserName);
    });

    it('should publish "Contacts" contract', async function it() {
      this.timeout(200000);

      // 1. Create schema
      const dapSchema = Object.assign({}, DashPay);
      dapSchema.title = `TestContacts_${bobUserName}`;

      // 2. Create contract
      const dapContract = Schema.create.dapcontract(dapSchema);
      const dapId = doubleSha256(Schema.serialize.encode(dapContract.dapcontract));

      // 3. Create ST packet
      let { stpacket: stPacket } = Schema.create.stpacket();
      stPacket = Object.assign(stPacket, dapContract);

      // 4. Create State Transition
      const transaction = new Transaction()
        .setType(Transaction.TYPES.TRANSACTION_SUBTX_TRANSITION);

      const serializedPacket = Schema.serialize.encode(stPacket);
      const stPacketHash = doubleSha256(serializedPacket);

      transaction.extraPayload
        .setRegTxId(bobRegTxId)
        .setHashPrevSubTx(bobRegTxId)
        .setHashSTPacket(stPacketHash)
        .setCreditFee(1000)
        .sign(bobPrivateKey);

      const transactionId = await dapiClient.sendRawTransition(
        transaction.serialize(),
        serializedPacket.toString('hex'),
      );

      expect(transactionId).to.be.a('string');
      expect(transactionId).to.be.not.empty();

      // 5. Mine transaction and Wait until Drive synced this block
      // await dapiClient.generate(1);
      await wait(150000);

      const dapContractFromDAPI = await dapiClient.fetchDapContract(dapId);

      expect(dapContractFromDAPI).to.have.property('dapName');
      expect(dapContractFromDAPI.dapName).to.be.equal(dapSchema.title);
    });

    xit('should create profile in "Contacts" app', async () => {

    });
  });

  xdescribe('Alice', () => {
    it('should register blockchain user', async () => {

    });
    it('should create profile in "Contacts" app', async () => {

    });
    it('should update only her profile', async () => {

    });
  });

  xdescribe('Bob', () => {
    it('should be able to send contact request', async () => {

    });
  });

  xdescribe('Alice', () => {
    it('should be able to approve contact request', async () => {

    });
    it('should be able to remove only here contact object', async () => {

    });
  });
});
