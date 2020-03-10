const sinon = require('sinon');

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

const DAPIClient = require('@dashevo/dapi-client');

const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const jayson = require('jayson/promise');

const IdentityCreateTransition = require('@dashevo/dpp/lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');

const Identity = require('@dashevo/dpp/lib/identity/Identity');


const DataContractStateTransition = require(
  '@dashevo/dpp/lib/dataContract/stateTransition/DataContractStateTransition',
);

const getDataContractFixture = require(
  '../../../../../lib/test/fixtures/getDataContractFixture',
);

const wait = require('../../../../../lib/utils/wait');

describe('rpcServer', function main() {
  this.timeout(200000);
  describe('applyStateTransitionHandlerFactory', () => {
    let removeDapi;
    let dapiClient;
    let driveClient;
    let stateTransition;
    let identityCreateTransition;
    let identityPublicKey;
    let privateKey;
    let dapiJsonRpcClient;
    let forcedClient;

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

      forcedClient = new DAPIClient({
        seeds: [{ service: dapiClient.MNDiscovery.masternodeListProvider.seeds[0].service }],
        forceJsonRpc: true,
        port: dapiClient.DAPIPort,
      });

      sinon.stub(forcedClient.MNDiscovery, 'getRandomMasternode').returns(
        { service: dapiClient.MNDiscovery.masternodeListProvider.seeds[0].service },
      );
    });

    afterEach(async () => {
      await removeDapi();
    });

    it('should register new identity and store contract', async () => {
      const dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());

      stateTransition = new DataContractStateTransition(dataContract);
      stateTransition.sign(identityPublicKey, privateKey);

      let { result } = await dapiJsonRpcClient.request('applyStateTransition', {
        stateTransition: identityCreateTransition.serialize().toString('base64'),
      });

      expect(result).to.be.true();

      ({ result } = await dapiJsonRpcClient.request('applyStateTransition', {
        stateTransition: stateTransition.serialize().toString('base64'),
      }));

      const contractId = stateTransition.getDataContract().getId();
      // Todo: fetch it from dapi too
      const { result: contract } = await driveClient.request('fetchContract', { contractId });

      expect(result).to.be.true();
      expect(contract).to.deep.equal(stateTransition.getDataContract().toJSON());
    });

    it('should respond with an error if contract is invalid', async () => {
      const dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());
      const unsignedStateTransition = new DataContractStateTransition(dataContract);

      const { result } = await dapiJsonRpcClient.request('applyStateTransition', {
        stateTransition: identityCreateTransition.serialize().toString('base64'),
      });

      expect(result).to.be.true();

      const response = await dapiJsonRpcClient.request('applyStateTransition', {
        stateTransition: unsignedStateTransition.serialize().toString('base64'),
      });

      expect(response.result).to.be.undefined();

      expect(response.error).to.be.deep.equal({ code: -32602, message: 'State Transition is invalid' });
    });

    it('client should return same result as grpc method', async () => {
      expect(forcedClient.forceJsonRpc).to.be.true();
      const dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());

      stateTransition = new DataContractStateTransition(dataContract);
      stateTransition.sign(identityPublicKey, privateKey);

      const result = await dapiClient.applyStateTransition(identityCreateTransition);

      expect(result).to.be.an.instanceOf(ApplyStateTransitionResponse);

      const resultFromForcedClient = await forcedClient.applyStateTransition(stateTransition);

      expect(resultFromForcedClient).to.be.an.instanceOf(ApplyStateTransitionResponse);

      const contractId = stateTransition.getDataContract().getId();
      const { result: contract } = await driveClient.request('fetchContract', { contractId });

      expect(contract).to.deep.equal(stateTransition.getDataContract().toJSON());
    });
  });
});
