const addSTPacketFactory = require('../../../lib/storage/ipfs/addSTPacketFactory');
const getStateTransitionPackets = require('../../../lib/test/fixtures/getTransitionPacketFixtures');

const registerUser = require('../../../lib/test/registerUser');
const createSTHeader = require('../../../lib/test/createSTHeader');

const startDashDriveInstance = require('../../../lib/test/services/dashDrive/startDashDriveInstance');
const startDashCoreInstance = require('../../../lib/test/services/dashCore/startDashCoreInstance');
const startMongoDbInstance = require('../../../lib/test/services/mongoDb/startMongoDbInstance');
const startIPFSInstance = require('../../../lib/test/services/IPFS/startIPFSInstance');

const createDashDriveInstance = require('../../../lib/test/services/dashDrive/createDashDriveInstance');

const wait = require('../../../lib/test/util/wait');
const cbor = require('cbor');

/**
 * Await Dash Core instance to finish syncing
 *
 * @param {DashCoreInstance} instance
 * @returns {Promise<void>}
 */
async function dashCoreSyncToFinish(instance) {
  let finished = false;
  while (!finished) {
    const status = await instance.getApi().mnsync('status');
    if (status.result.IsSynced) {
      finished = true;
    } else {
      await wait(3000);
    }
  }
}

/**
 * Await Dash Drive instance to finish syncing
 *
 * @param {DashDriveInstance} instance
 * @returns {Promise<void>}
 */
async function dashDriveSyncToFinish(instance) {
  const packet = getStateTransitionPackets()[0];
  const serializedPacket = cbor.encodeCanonical(packet.toJSON({ skipMeta: true }));
  const serializedPacketJson = {
    packet: serializedPacket.toString('hex'),
  };

  let finished = false;
  while (!finished) {
    try {
      const response = await instance.getApi()
        .request('addSTPacket', serializedPacketJson);
      if (response.result) {
        finished = true;
      } else {
        await wait(1000);
      }
    } catch (e) {
      await wait(1000);
    }
  }
}

describe('Initial sync of Dash Drive and Dash Core', function main() {
  // First node
  let fullDashDriveInstance;

  // Second node
  let dashCoreInstance;
  let mongoDbInstance;
  let dashDriveStandaloneInstance;
  let ipfsInstance;

  let packetsCids;
  let packetsData;

  this.timeout(900000);

  before('having Dash Drive node #1 up and ready, some amount of STs generated and Dash Drive on node #1 fully synced', async () => {
    packetsCids = [];
    packetsData = getStateTransitionPackets();

    // 1. Start first Dash Drive node
    fullDashDriveInstance = await startDashDriveInstance();

    // 1.1 Activate Special Transactions
    await fullDashDriveInstance.dashCore.getApi().generate(1000);

    // 2. Populate Dash Drive and Dash Core With data
    async function createAndSubmitST(username) {
      // 2.1 Get packet data with random object description
      const packetOne = packetsData[0];
      packetOne.dapcontract.description = `Valid registration for ${username}`;

      // 2.2 Register user and create DAP Contract State Transition packet and header
      const { userId, privateKeyString } =
        await registerUser(username, fullDashDriveInstance.dashCore.getApi());
      const header = await createSTHeader(userId, privateKeyString, packetOne);

      // 2.3 Add ST packet to IPFS
      const addSTPacket = addSTPacketFactory(fullDashDriveInstance.ipfs.getApi());
      const packetCid = await addSTPacket(packetOne);

      // 2.4 Save CID of freshly added packet for future use
      packetsCids.push(packetCid);

      // 2.5 Send ST header to Dash Core and generate a block with it
      await fullDashDriveInstance.dashCore.getApi().sendRawTransition(header.serialize());
      await fullDashDriveInstance.dashCore.getApi().generate(1);
    }

    // Note: I can't use Promise.all here due to errors with PrivateKey
    //       I guess some of the actions can't be executed in parallel
    for (let i = 0; i < 4; i++) {
      await createAndSubmitST(`Alice_${i}`);
    }
  });

  it('Dash Drive should sync the data with Dash Core upon startup', async () => {
    // 3. Start services of the 2nd node (Core, Mongo, IPFS),
    //    but without Drive as we have to be sure Core is synced first
    dashCoreInstance = await startDashCoreInstance();

    await dashCoreInstance.connect(fullDashDriveInstance.dashCore);

    mongoDbInstance = await startMongoDbInstance();

    ipfsInstance = await startIPFSInstance();
    await ipfsInstance.connect(fullDashDriveInstance.ipfs);

    // 4. Await Dash Core to finish syncing
    await dashCoreSyncToFinish(dashCoreInstance);

    // 5. Start Dash Drive on 2nd node
    const envs = [
      `DASHCORE_ZMQ_PUB_HASHBLOCK=${dashCoreInstance.getZmqSockets().hashblock}`,
      `DASHCORE_JSON_RPC_HOST=${dashCoreInstance.getIp()}`,
      `DASHCORE_JSON_RPC_PORT=${dashCoreInstance.options.getRpcPort()}`,
      `DASHCORE_JSON_RPC_USER=${dashCoreInstance.options.getRpcUser()}`,
      `DASHCORE_JSON_RPC_PASS=${dashCoreInstance.options.getRpcPassword()}`,
      `STORAGE_IPFS_MULTIADDR=${ipfsInstance.getIpfsAddress()}`,
      `STORAGE_MONGODB_URL=mongodb://${mongoDbInstance.getIp()}:27017`,
    ];

    dashDriveStandaloneInstance = await createDashDriveInstance(envs);
    await dashDriveStandaloneInstance.start();

    // 6. Await Dash Drive on the 2nd node to finish syncing
    await dashDriveSyncToFinish(dashDriveStandaloneInstance);

    // 7. Get all pinned CIDs on the 2nd node and assert
    //    they contain CIDs saved from the 1st node
    const lsResult = await ipfsInstance.getApi().pin.ls();

    const hashes = lsResult.map(item => item.hash);

    expect(hashes).to.contain.members(packetsCids);
  });

  after('cleanup lone services', async () => {
    const instances = [
      mongoDbInstance,
      dashCoreInstance,
      fullDashDriveInstance,
      dashDriveStandaloneInstance,
      ipfsInstance,
    ];

    await Promise.all(instances.filter(i => i)
      .map(i => i.remove()));
  });
});
