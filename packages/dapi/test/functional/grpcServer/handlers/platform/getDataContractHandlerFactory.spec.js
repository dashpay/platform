const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');

const getDataContractFixture = require(
  '@dashevo/dpp/lib/test/fixtures/getDataContractFixture',
);

const wait = require('../../../../../lib/utils/wait');

describe('getDataContractHandlerFactory', function main() {
  this.timeout(200000);

  let removeDapi;
  let dapiClient;
  let dpp;
  let identityCreateTransition;
  let publicKeyId;
  let dataContract;

  before(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi({});

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

    // eslint-disable-next-line no-underscore-dangle
    const publicKeyHash = PublicKey.fromBuffer(Buffer.from(pubKeyBase, 'base64'))._getID();
    publicKeyId = 0;

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

    const identity = dpp.identity.create(
      outPoint,
      [publicKey],
    );

    identityCreateTransition = dpp.identity.createIdentityCreateTransition(identity);
    identityCreateTransition.signByPrivateKey(privateKey);

    dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());

    const dataContractCreateTransition = dpp.dataContract.createStateTransition(dataContract);
    dataContractCreateTransition.sign(identity.getPublicKeyById(publicKeyId), privateKey);

    // Create Identity
    await dapiClient.platform.broadcastStateTransition(identityCreateTransition.serialize());

    // Create Data Contract
    await dapiClient.platform.broadcastStateTransition(dataContractCreateTransition.serialize());
  });

  after(async () => {
    await removeDapi();
  });

  it('should fetch created data contract', async () => {
    const serializedDataContract = await dapiClient.platform.getDataContract(
      dataContract.getId(),
    );

    expect(serializedDataContract).to.be.not.null();

    const receivedDataContract = await dpp.dataContract.createFromSerialized(
      serializedDataContract,
      { skipValidation: true },
    );

    expect(dataContract.toJSON()).to.deep.equal(receivedDataContract.toJSON());
  });

  it('should respond with null if data contract not found', async () => {
    const serializedDataContract = await dapiClient.platform.getDataContract(
      'unknownId',
    );

    expect(serializedDataContract).to.be.null();
  });
});
