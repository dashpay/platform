const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const { convertSatoshiToCredits } = require(
  '@dashevo/dpp/lib/identity/creditsConverter',
);

const wait = require('../../../../../lib/utils/wait');

describe('topUpIdentity', function main() {
  this.timeout(200000);

  let removeDapi;
  let dapiClient;
  let dpp;
  let identity;
  let identityCreateTransition;
  let identityTopUpTransition;
  let coreAPI;
  let addressString;
  let publicKeyHash;
  let privateKey;

  before(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;
    coreAPI = dashCore.getApi();
    dapiClient = dapiCore.getApi();

    dpp = new DashPlatformProtocol({
      dataProvider: {},
    });
  });

  beforeEach(async () => {
    ({ result: addressString } = await coreAPI.getNewAddress());
    const { result: privateKeyString } = await coreAPI.dumpPrivKey(addressString);

    privateKey = new PrivateKey(privateKeyString);
    const publicKey = new PublicKey({
      ...privateKey.toPublicKey().toObject(),
      compressed: true,
    });
    const pubKeyBase = publicKey.toBuffer()
      .toString('base64');

    // eslint-disable-next-line no-underscore-dangle
    publicKeyHash = PublicKey.fromBuffer(Buffer.from(pubKeyBase, 'base64'))
      ._getID();

    await coreAPI.generateToAddress(500, addressString);

    const { result: unspent } = await coreAPI.listUnspent();
    const inputs = unspent.filter(input => input.address === addressString);

    const transaction = new Transaction();

    transaction.from(inputs.slice(-1)[0])
      .addBurnOutput(10000, publicKeyHash)
      .change(addressString)
      .fee(668)
      .sign(privateKey);

    await coreAPI.sendrawtransaction(transaction.serialize());

    await coreAPI.generateToAddress(1, addressString);

    await wait(2000); // wait a couple of seconds for tx to be confirmed

    const outPoint = transaction.getOutPointBuffer(0);

    identity = dpp.identity.create(
      outPoint,
      [publicKey],
    );

    identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);
    identityCreateTransition.signByPrivateKey(privateKey);

    await dapiClient.applyStateTransition(identityCreateTransition);
  });

  after(async () => {
    await removeDapi();
  });

  it('should top up created identity', async () => {
    const { result: unspent } = await coreAPI.listUnspent();
    const inputs = unspent.filter(input => input.address === addressString);
    const topUpTransaction = new Transaction();
    const topUpAmount = 3000;

    topUpTransaction.from(inputs.slice(-1)[0])
      .addBurnOutput(topUpAmount, publicKeyHash)
      .change(addressString)
      .fee(668)
      .sign(privateKey);

    await coreAPI.sendrawtransaction(topUpTransaction.serialize());

    await coreAPI.generateToAddress(1, addressString);

    await wait(2000); // wait a couple of seconds for tx to be confirmed

    const topUpOutPoint = topUpTransaction.getOutPointBuffer(0);

    identityTopUpTransition = dpp.identity.createIdentityTopUpTransition(
      identity.getId(),
      topUpOutPoint,
    );
    identityTopUpTransition.signByPrivateKey(privateKey);

    await dapiClient.applyStateTransition(identityTopUpTransition);

    const serializedIdentity = await dapiClient.getIdentity(
      identityCreateTransition.getIdentityId(),
    );

    const receivedIdentity = dpp.identity.createFromSerialized(
      serializedIdentity,
      { skipValidation: true },
    );

    const balance = convertSatoshiToCredits(10000)
    + convertSatoshiToCredits(topUpAmount)
    - identityCreateTransition.calculateFee()
    - identityTopUpTransition.calculateFee();

    expect(balance).to.equal(receivedIdentity.getBalance());
  });

  it('should fail top up created identity ', async () => {
    const { result: unspent } = await coreAPI.listUnspent();
    const inputs = unspent.filter(input => input.address === addressString);
    const topUpTransaction = new Transaction();
    const topUpAmount = 3000;

    topUpTransaction.from(inputs.slice(-1)[0])
      .addBurnOutput(topUpAmount, publicKeyHash)
      .change(addressString)
      .fee(668)
      .sign(privateKey);

    const topUpOutPoint = topUpTransaction.getOutPointBuffer(0);

    identityTopUpTransition = dpp.identity.createIdentityTopUpTransition(
      identity.getId(),
      topUpOutPoint,
    );
    identityTopUpTransition.signByPrivateKey(privateKey);

    try {
      await dapiClient.applyStateTransition(identityTopUpTransition);

      expect.fail('Should fail with error');
    } catch (e) {
      expect(e.code).to.equal(GrpcErrorCodes.INVALID_ARGUMENT);
      expect(e.details).to.equal('State Transition is invalid');
    }

    const serializedIdentity = await dapiClient.getIdentity(
      identityCreateTransition.getIdentityId(),
    );

    const receivedIdentity = dpp.identity.createFromSerialized(
      serializedIdentity,
      { skipValidation: true },
    );

    expect(convertSatoshiToCredits(10000) - identityCreateTransition.calculateFee())
      .to.equal(receivedIdentity.getBalance());
  });
});
