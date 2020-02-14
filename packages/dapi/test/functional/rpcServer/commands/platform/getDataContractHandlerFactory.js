const {
  startDapi,
} = require('@dashevo/dp-services-ctl');

const jayson = require('jayson/promise');
const sinon = require('sinon');

const {
  PrivateKey,
  PublicKey,
  Transaction,
} = require('@dashevo/dashcore-lib');
const DAPIClient = require('@dashevo/dapi-client');

const DashPlatformProtocol = require('@dashevo/dpp');

const IdentityPublicKey = require(
  '@dashevo/dpp/lib/identity/IdentityPublicKey',
);

const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const Identity = require('@dashevo/dpp/lib/identity/Identity');

const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');
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
    let dataContractStateTransition;
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
        identityType: Identity.TYPES.APPLICATION,
        publicKeys,
      }, { skipValidation: true });

      const identityPublicKey = new IdentityPublicKey()
        .setId(publicKeyId)
        .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
        .setData(pubKeyBase);

      identityCreateTransition.sign(identityPublicKey, privateKey);

      await dapiClient.applyStateTransition(identityCreateTransition);

      const dataContract = getDataContractFixture(identityCreateTransition.getIdentityId());
      dataContractStateTransition = dpp.dataContract
        .createStateTransition(dataContract)
        .sign(identityPublicKey, privateKey);

      await dapiClient.applyStateTransition(dataContractStateTransition);

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

    it('should fetch created data contract', async () => {
      const response = await dapiRpc.request('getDataContract', { id: identityCreateTransition.getIdentityId() });

      const fetchedContract = dpp.dataContract.createFromSerialized(
        Buffer.from(response.result.dataContract, 'base64'),
        { skipValidation: true },
      );
      expect(
        getDataContractFixture(identityCreateTransition.getIdentityId()).toJSON(),
      ).to.deep.equal(dpp.dataContract.createFromObject(fetchedContract).toJSON());
    });

    it('should return the same result as grpc client', async () => {
      const contractFromForcedClient = await forcedClient.getDataContract(
        identityCreateTransition.getIdentityId(),
      );
      const contractFromGrpc = await dapiClient.getDataContract(
        identityCreateTransition.getIdentityId(),
      );

      expect(contractFromForcedClient).to.be.deep.equal(contractFromGrpc);

      const fetchedContract = dpp.dataContract.createFromSerialized(
        contractFromForcedClient,
        { skipValidation: true },
      );
      expect(
        getDataContractFixture(identityCreateTransition.getIdentityId()).toJSON(),
      ).to.deep.equal(dpp.dataContract.createFromObject(fetchedContract).toJSON());
    });

    it('should respond with an error if contract not found', async () => {
      const response = await dapiRpc.request('getDataContract', { id: 'FQJtYcWqM8mMvTMerJNTixMPqbp8qdYm8uESp6DVQNFK' });

      expect(response.result).to.be.undefined();
      expect(response.error).to.be.deep.equal({ code: -32602, message: 'Contract not found' });
    });
  });
});
