const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');
const DAPIClient = require('@dashevo/dapi-client');

const IdentityPublicKey = require(
  '@dashevo/dpp/lib/identity/IdentityPublicKey',
);

const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const jayson = require('jayson/promise');
const sinon = require('sinon');

const IdentityCreateTransition = require('@dashevo/dpp/lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');

const Identity = require('@dashevo/dpp/lib/identity/Identity');
const DPP = require('@dashevo/dpp');

const DataContractStateTransition = require(
  '@dashevo/dpp/lib/dataContract/stateTransition/DataContractStateTransition',
);

const getDataContractFixture = require(
  '../../../../../lib/test/fixtures/getDataContractFixture',
);

const wait = require('../../../../../lib/utils/wait');

describe('applyStateTransitionHandlerFactory', function main() {
  this.timeout(200000);

  let dpp;
  let removeDapi;
  let dapiClient;
  let stateTransition;
  let identityCreateTransition;
  let identityPublicKey;
  let privateKey;
  let dapiJsonRpcClient;
  let dataContract;
  let contractId;
  let forcedClient;

  beforeEach(async () => {
    const {
      dapiCore,
      dashCore,
      remove,
    } = await startDapi();

    removeDapi = remove;

    dapiClient = dapiCore.getApi();
    const coreAPI = dashCore.getApi();

    dpp = new DPP();

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

    dapiJsonRpcClient = jayson.client.http({
      host: dapiClient.MNDiscovery.masternodeListProvider.seeds[0].service,
      port: dapiClient.DAPIPort,
    });

    dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());

    stateTransition = new DataContractStateTransition(dataContract);
    stateTransition.sign(identityPublicKey, privateKey);

    await dapiJsonRpcClient.request('applyStateTransition', {
      stateTransition: identityCreateTransition.serialize().toString('base64'),
    });

    await dapiJsonRpcClient.request('applyStateTransition', {
      stateTransition: stateTransition.serialize().toString('base64'),
    });

    contractId = stateTransition.getDataContract().getId();

    forcedClient = new DAPIClient({
      seeds: [{ service: dapiClient.MNDiscovery.masternodeListProvider.seeds[0].service }],
      forceJsonRpc: true,
      port: dapiClient.DAPIPort,
    });

    sinon.stub(forcedClient.MNDiscovery, 'getRandomMasternode').returns(
      {
        service: dapiClient.MNDiscovery.masternodeListProvider.seeds[0].service,
        getIp() {
          return dapiClient.MNDiscovery.masternodeListProvider.seeds[0].service.split(':')[0];
        },
      },
    );
  });

  afterEach(async () => {
    await removeDapi();
  });

  it('should fetch documents', async () => {
    const docType = 'indexedDocument';
    const documents = [{ type: docType, data: { firstName: 'Test', lastName: 'Demo' } }];
    const identityId = identityCreateTransition.getIdentityId();

    const documentModels = documents.map(
      doc => dpp.document.create(
        dataContract, identityId, doc.type, doc.data,
      ),
    );
    const documentsStateTransition = dpp.document.createStateTransition(documentModels);
    documentsStateTransition.sign(identityPublicKey, privateKey);

    await dapiClient.applyStateTransition(documentsStateTransition);

    await wait(5000);

    const response = await dapiJsonRpcClient.request('getDocuments', {
      dataContractId: contractId,
      documentType: docType,
      where: [['$userId', '==', identityId]],
    });

    expect(response.result.length).to.be.equal(1);
    expect(response.result[0]).to.be.deep.equal(documentModels[0].toJSON());
  });

  it('forced client should give the same result as grpc method', async () => {
    const docType = 'indexedDocument';
    const documents = [{ type: docType, data: { firstName: 'Test', lastName: 'Demo' } }];
    const identityId = identityCreateTransition.getIdentityId();

    const documentModels = documents.map(
      doc => dpp.document.create(
        dataContract, identityId, doc.type, doc.data,
      ),
    );
    const documentsStateTransition = dpp.document.createStateTransition(documentModels);
    documentsStateTransition.sign(identityPublicKey, privateKey);

    await dapiClient.applyStateTransition(documentsStateTransition);

    await wait(5000);

    const options = { where: [['$userId', '==', identityId]] };
    const documentsFromForcedClient = await forcedClient.getDocuments(contractId, docType, options);

    expect(documentsFromForcedClient.length).to.be.equal(1);
    expect(
      documentsFromForcedClient[0],
    ).to.be.deep.equal(
      documentModels[0].serialize(),
    );
  });
});
