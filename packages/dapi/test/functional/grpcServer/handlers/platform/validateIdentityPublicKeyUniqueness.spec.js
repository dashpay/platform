const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const wait = require('../../../../../lib/utils/wait');

/**
 * @param {RpcClient} coreAPI
 * @param {string} address
 * @param {PublicKey} publicKey
 * @param {PrivateKey} privateKey
 *
 * @returns {Promise<Buffer>}
 */
async function getOutPointBuffer(coreAPI, address, publicKey, privateKey) {
  const pubKeyBase = publicKey.toBuffer()
    .toString('base64');

  // eslint-disable-next-line no-underscore-dangle
  const publicKeyHash = PublicKey.fromBuffer(Buffer.from(pubKeyBase, 'base64'))
    ._getID();

  await coreAPI.generateToAddress(500, address);

  const { result: unspent } = await coreAPI.listUnspent();
  const inputs = unspent.filter(input => input.address === address);

  const transaction = new Transaction();

  transaction.from(inputs.slice(-1)[0])
    .addBurnOutput(10000, publicKeyHash)
    .change(address)
    .fee(668)
    .sign(privateKey);

  await coreAPI.sendRawTransaction(transaction.serialize());
  await coreAPI.generateToAddress(1, address);

  // wait a couple of seconds for tx to be confirmed
  await wait(2000);

  return transaction.getOutPointBuffer(0);
}

describe('validateIdentityPublicKeyUniqueness', function main() {
  this.timeout(200000);

  let removeDapi;
  let dapiClient;
  let dpp;
  let identityCreateTransition;
  let identity;
  let publicKey;
  let privateKey;
  let addressString;
  let coreAPI;

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

    ({ result: addressString } = await coreAPI.getNewAddress());
    const { result: privateKeyString } = await coreAPI.dumpPrivKey(addressString);

    privateKey = new PrivateKey(privateKeyString);
    publicKey = new PublicKey({
      ...privateKey.toPublicKey().toObject(),
      compressed: true,
    });

    const outPoint = await getOutPointBuffer(
      coreAPI, addressString, publicKey, privateKey,
    );

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

  it('should throw error in case public key is already taken', async () => {
    const outPoint = await getOutPointBuffer(
      coreAPI, addressString, publicKey, privateKey,
    );

    identity = dpp.identity.create(
      outPoint,
      [publicKey],
    );

    identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);
    identityCreateTransition.signByPrivateKey(privateKey);

    try {
      await dapiClient.applyStateTransition(identityCreateTransition);
      expect.fail('Error was not thrown');
    } catch (e) {
      const [error] = JSON.parse(e.metadata.get('errors')[0]);
      expect(error.name).to.equal('IdentityFirstPublicKeyAlreadyExistsError');
      expect(error.publicKeyHash).to.equal(publicKey.hash.toString('hex'));
    }
  });
});
