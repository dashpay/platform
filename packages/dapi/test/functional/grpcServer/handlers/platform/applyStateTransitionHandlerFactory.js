const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  ApplyStateTransitionResponse,
} = require('@dashevo/dapi-grpc');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const IdentityPublicKey = require(
  '@dashevo/dpp/lib/identity/IdentityPublicKey',
);

const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const IdentityCreateTransition = require('@dashevo/dpp/lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');

const Identity = require('@dashevo/dpp/lib/identity/Identity');


const DataContractStateTransition = require(
  '@dashevo/dpp/lib/dataContract/stateTransition/DataContractStateTransition',
);

const getDataContractFixture = require(
  '../../../../../lib/test/fixtures/getDataContractFixture',
);

const wait = require('../../../../../lib/utils/wait');

describe('applyStateTransitionHandlerFactory', function main() {
  this.timeout(200000);

  let removeDapi;
  let dapiClient;
  let driveClient;
  let stateTransition;
  let identityCreateTransition;
  let identityPublicKey;
  let privateKey;

  beforeEach(async () => {
    const {
      driveApi,
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    dapiClient = dapiCore.getApi();
    driveClient = driveApi.getApi();
    const coreAPI = dashCore.getApi();

    const { result: addressString } = await coreAPI.getNewAddress();
    const { result: privateKeyString } = await coreAPI.dumpPrivKey(addressString);

    privateKey = new PrivateKey(privateKeyString);
    const pubKeyBase = new PublicKey({
      ...privateKey.toPublicKey().toObject(),
      compressed: true,
    }).toBuffer()
      .toString('base64');

    // eslint-disable-next-line no-underscore-dangle
    const publicKeyHash = PublicKey.fromBuffer(Buffer.from(pubKeyBase, 'base64'))._getID();
    const publicKeyId = 1;

    await coreAPI.generate(500);
    await coreAPI.sendToAddress(addressString, 10);
    await coreAPI.generate(10);

    const { result: unspent } = await coreAPI.listUnspent();
    const inputs = unspent.filter(input => input.address === addressString);

    const transaction = new Transaction();

    transaction.from(inputs)
      .addBurnOutput(10000, publicKeyHash)
      .change(addressString)
      .fee(668)
      .sign(privateKey);

    await coreAPI.sendrawtransaction(transaction.serialize());

    await coreAPI.generate(1);

    await wait(2000); // wait a couple of seconds for tx to be confirmed

    const outPoint = transaction.getOutPointBuffer(0)
      .toString('base64');

    identityCreateTransition = new IdentityCreateTransition({
      protocolVersion: 0,
      type: stateTransitionTypes.IDENTITY_CREATE,
      lockedOutPoint: outPoint,
      identityType: Identity.TYPES.APPLICATION,
      publicKeys: [
        {
          id: publicKeyId,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: pubKeyBase,
          isEnabled: true,
        },
      ],
    });

    identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(pubKeyBase);

    identityCreateTransition.sign(identityPublicKey, privateKey);
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should register new identity and store contract', async () => {
    const dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());

    stateTransition = new DataContractStateTransition(dataContract);
    stateTransition.sign(identityPublicKey, privateKey);

    let result = await dapiClient.applyStateTransition(identityCreateTransition);

    expect(result).to.be.an.instanceOf(ApplyStateTransitionResponse);

    result = await dapiClient.applyStateTransition(stateTransition);

    const contractId = stateTransition.getDataContract().getId();
    const { result: contract } = await driveClient.request('fetchContract', { contractId });

    expect(result).to.be.an.instanceOf(ApplyStateTransitionResponse);
    expect(contract).to.deep.equal(stateTransition.getDataContract().toJSON());
  });

  it('should respond with error if contract is invalid', async () => {
    const dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());
    const unsignedStateTransition = new DataContractStateTransition(dataContract);

    const result = await dapiClient.applyStateTransition(identityCreateTransition);

    expect(result).to.be.an.instanceOf(ApplyStateTransitionResponse);

    try {
      await dapiClient.applyStateTransition(unsignedStateTransition);

      expect.fail('should throw an error');
    } catch (e) {
      const [error] = JSON.parse(e.metadata.get('errors')[0]);

      expect(error.name).to.equal('JsonSchemaError');
      expect(error.dataPath).to.equal('.signature');
    }
  });
});
