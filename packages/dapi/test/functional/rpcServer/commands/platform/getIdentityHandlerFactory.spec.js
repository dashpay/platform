const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const sinon = require('sinon');
const jayson = require('jayson/promise');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('@dashevo/dpp');
const DAPIClient = require('@dashevo/dapi-client');

const IdentityPublicKey = require(
  '@dashevo/dpp/lib/identity/IdentityPublicKey',
);

const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const Identity = require('@dashevo/dpp/lib/identity/Identity');

const wait = require('../../../../../lib/utils/wait');

describe('rpcServer', function main() {
  this.timeout(320000);

  describe('getIdentityHandlerFactory', () => {
    let removeDapi;
    let dapiClient;
    let dpp;
    let identityCreateTransition;
    let publicKeys;
    let publicKeyId;
    let dapiRpc;
    let forcedClient;

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

      dapiRpc = jayson.client.http({
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

    it('should fetch created identity', async () => {
      const response = await dapiRpc.request('getIdentity', { id: identityCreateTransition.getIdentityId() });

      const createdIdentity = dpp.identity.applyIdentityStateTransition(
        identityCreateTransition,
        null,
      );
      const identity = dpp.identity.createFromSerialized(Buffer.from(response.result.identity, 'base64'), { skipValidation: true });
      expect(createdIdentity.toJSON()).to.deep.equal(identity.toJSON());
    });

    it('should respond with null if identity not found', async () => {
      const response = await dapiRpc.request('getIdentity', { id: 'unknownId' });

      expect(response.result).to.be.undefined();
    });

    it('should return same result as grpc method', async () => {
      const identityNotForced = await dapiClient.getIdentity(
        identityCreateTransition.getIdentityId(),
      );
      const identityFromForcedClient = await forcedClient.getIdentity(
        identityCreateTransition.getIdentityId(),
      );

      expect(identityNotForced).to.be.deep.equal(identityFromForcedClient);

      const createdIdentity = dpp.identity.applyIdentityStateTransition(
        identityCreateTransition,
        null,
      );
      const identity = dpp.identity.createFromSerialized(identityFromForcedClient, {
        skipValidation: true,
      });
      expect(createdIdentity.toJSON()).to.deep.equal(identity.toJSON());
    });
  });
});
