const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const wait = require('../../../../../lib/utils/wait');

describe('getDocumentsHandlerFactory', function main() {
  this.timeout(90000);

  let removeDapi;
  let dpp;
  let dapiClient;
  let identity;
  let dataContract;
  let identityPrivateKey;

  before(async () => {
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
    const publicKey = new PublicKey({
      ...privateKey.toPublicKey().toObject(),
      compressed: true,
    });
    const pubKeyBase = publicKey.toBuffer()
      .toString('base64');

    identityPrivateKey = privateKey;

    // eslint-disable-next-line no-underscore-dangle
    const publicKeyHash = PublicKey.fromBuffer(Buffer.from(pubKeyBase, 'base64'))
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

    const identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);
    identityCreateTransition.signByPrivateKey(privateKey);

    await dapiClient.applyStateTransition(identityCreateTransition);

    dataContract = getDataContractFixture(identity.getId());

    const dataContractStateTransition = dpp.dataContract.createStateTransition(dataContract);
    dataContractStateTransition.sign(identity.getPublicKeyById(1), identityPrivateKey);

    await dapiClient.applyStateTransition(dataContractStateTransition);
  });

  after(async () => {
    await removeDapi();
  });

  it('should fetch created documents array', async () => {
    const document = dpp.document.create(
      dataContract, identity.getId(), 'niceDocument', {
        name: 'someName',
      },
    );

    const documentTransition = dpp.document.createStateTransition({
      create: [document],
    });
    documentTransition.sign(identity.getPublicKeyById(1), identityPrivateKey);

    await dapiClient.applyStateTransition(documentTransition);

    const [documentBuffer] = await dapiClient.getDocuments(dataContract.getId(), 'niceDocument', {});

    const receivedDocument = await dpp.document.createFromSerialized(
      documentBuffer, { skipValidation: true },
    );

    expect(document.toJSON()).to.deep.equal(receivedDocument.toJSON());
  });
});
