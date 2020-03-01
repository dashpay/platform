const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const IdentityPublicKey = require(
  '@dashevo/dpp/lib/identity/IdentityPublicKey',
);

const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const Identity = require('@dashevo/dpp/lib/identity/Identity');

const wait = require('../../../../../lib/utils/wait');

describe('getIdentityHandlerFactory', function main() {
  this.timeout(200000);

  let removeDapi;
  let dapiClient;
  let dpp;
  let identityCreateTransition;
  let publicKeys;
  let publicKeyId;

  beforeEach(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    const coreAPI = dashCore.getApi();
    dapiClient = dapiCore.getApi();

    dpp = new DashPlatformProtocol({
      dataProvider: {},
    });

    const { result: addressString } = await coreAPI.getNewAddress();
    const { result: privateKeyString } = await coreAPI.dumpPrivKey(addressString);

    const privateKey = new PrivateKey(privateKeyString);
    const pubKeyBase = new PublicKey({
      ...privateKey.toPublicKey().toObject(),
      compressed: true,
    }).toBuffer()
      .toString('base64');

    // eslint-disable-next-line no-underscore-dangle
    const publicKeyHash = PublicKey.fromBuffer(Buffer.from(pubKeyBase, 'base64'))._getID();
    publicKeyId = 1;

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

    const outPoint = transaction.getOutPointBuffer(0)
      .toString('base64');

    publicKeys = [
      {
        id: publicKeyId,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: pubKeyBase,
        isEnabled: true,
      },
    ];

    identityCreateTransition = await dpp.stateTransition.createFromObject({
      protocolVersion: 0,
      type: stateTransitionTypes.IDENTITY_CREATE,
      lockedOutPoint: outPoint,
      identityType: Identity.TYPES.USER,
      publicKeys,
    }, { skipValidation: true });

    const identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(pubKeyBase);

    identityCreateTransition.sign(identityPublicKey, privateKey);

    await dapiClient.applyStateTransition(identityCreateTransition);
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should fetch created identity', async () => {
    const serializedIdentity = await dapiClient.getIdentity(
      identityCreateTransition.getIdentityId(),
    );

    expect(serializedIdentity).to.be.not.null();

    const createdIdentity = dpp.identity.applyIdentityStateTransition(
      identityCreateTransition,
      null,
    );

    const identity = dpp.identity.createFromSerialized(
      serializedIdentity,
      { skipValidation: true },
    );

    expect(createdIdentity.toJSON()).to.deep.equal(identity.toJSON());
  });

  it('should respond with null if identity not found', async () => {
    const identity = await dapiClient.getIdentity('unknownId');

    expect(identity).to.be.null();
  });
});
