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
    const status = await instance.rpcClient.mnsync('status');
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
  const serializedPacket = cbor.encodeCanonical(packet);
  const serializedPacketJson = {
    packet: serializedPacket.toString('hex'),
  };

  let finished = false;
  while (!finished) {
    try {
      const response = await instance.getApi()
        .request('addSTPacketMethod', serializedPacketJson);
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

describe('Sync interruption and resume between Dash Drive and Dash Core', function main() {
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

  before('having Dash Drive node #1 up and running', async () => {
    // 1. Start first Dash Drive node
    fullDashDriveInstance = await startDashDriveInstance();

    packetsCids = [];
    packetsData = getStateTransitionPackets();

    // 2. Populate Dash Drive and Dash Core with data
    async function createAndSubmitST(username) {
      // 2.1 Get packet data with random object description
      const [packetOne] = packetsData;
      packetOne.dapcontract.description = `Valid registration for ${username}`;

      // 2.2 Register user and create DAP Contract State Transition packet and header
      const { userId, privateKeyString } =
        await registerUser(username, fullDashDriveInstance.dashCore.rpcClient);
      const header = await createSTHeader(userId, privateKeyString, packetOne);

      // 2.3 Add ST packet to IPFS
      const addSTPacket = addSTPacketFactory(fullDashDriveInstance.ipfs.getApi());
      const packetCid = await addSTPacket(packetOne);

      // 2.4 Save CID of frshly added packet for future use
      packetsCids.push(packetCid);

      // 2.5 Send ST header to Dash Core and generate a block with it
      await fullDashDriveInstance.dashCore.rpcClient.sendRawTransition(header);
      await fullDashDriveInstance.dashCore.rpcClient.generate(1);
    }

    // Note: I can't use Promise.all here due to errors with PrivateKey
    //       I guess some of the actions can't be executed in parallel
    for (let i = 0; i < 20; i++) {
      await createAndSubmitST(`Alice_${i}`);
    }
  });

  it('Dash Drive should save sync state and continue from saved point after resume', async () => {
    // 3. Start services of the 2nd node (Core, Mongo, IPFS),
    //    but without Drive as we have to be sure Core is synced first
    dashCoreInstance = await startDashCoreInstance();
    await dashCoreInstance.connect(fullDashDriveInstance.dashCore);

    mongoDbInstance = await startMongoDbInstance();

    ipfsInstance = await startIPFSInstance();
    await ipfsInstance.connect(fullDashDriveInstance.ipfs);

    // 4. Await Dash Core to finish syncing
    await dashCoreSyncToFinish(dashCoreInstance);

    const envs = [
      `DASHCORE_ZMQ_PUB_HASHBLOCK=${dashCoreInstance.getZmqSockets().hashblock}`,
      `DASHCORE_JSON_RPC_HOST=${dashCoreInstance.getIp()}`,
      `DASHCORE_JSON_RPC_PORT=${dashCoreInstance.options.getRpcPort()}`,
      `DASHCORE_JSON_RPC_USER=${dashCoreInstance.options.getRpcUser()}`,
      `DASHCORE_JSON_RPC_PASS=${dashCoreInstance.options.getRpcPassword()}`,
      `STORAGE_IPFS_MULTIADDR=${ipfsInstance.getIpfsAddress()}`,
      `STORAGE_MONGODB_URL=mongodb://${mongoDbInstance.getIp()}:27017`,
    ];

    // 7. Save initial list of CIDs in IPFS before Dash Drive started on 2nd node
    let lsResult = await ipfsInstance.getApi().pin.ls();
    const initialHashes = lsResult.map(item => item.hash);

    // 6. Start Dash Drive on 2nd node
    dashDriveStandaloneInstance = await createDashDriveInstance(envs);
    await dashDriveStandaloneInstance.start();

    // 7. Wait for IPFS on 2nd node to have 3 packets pinned
    //    Wait maximum of 60 seconds in total

    // TODO: implement this bit in the future using
    //       getSyncStatus API method of Dash Drive
    //       possibly implemented in DD-196
    for (let i = 0; i < 60; i++) {
      lsResult = await ipfsInstance.getApi().pin.ls();
      const pinnedHashes = lsResult
        .filter(item => initialHashes.indexOf(item.hash) === -1)
        .map(item => item.hash);
      if (pinnedHashes.length >= 3) {
        break;
      }
      await wait(1000);
    }

    // 8. Stop Dash Drive on 2nd node
    await dashDriveStandaloneInstance.stop();

    // 9. Save a list of CIDs pinned on 2nd node
    //    Filter out initial CIDs from step #7
    //    to have a clean list of freshly pinned CIDs
    //    as a result of sync process
    lsResult = await ipfsInstance.getApi().pin.ls();
    const pinnedHashes = lsResult
      .filter(item => initialHashes.indexOf(item.hash) === -1)
      .map(item => item.hash);

    // 10. Remove freshly pinned CIDs
    //     This will allow us to check
    //     sync started from the point it stopped
    const rmPromises = Promise
      .all(pinnedHashes.map(hash => ipfsInstance.getApi().pin.rm(hash)));
    await rmPromises;

    // 11. Start Dash Drive on 2nd node
    await dashDriveStandaloneInstance.start();

    // 12. Await Dash Drive to finish the rest of synchronisation
    await dashDriveSyncToFinish(dashDriveStandaloneInstance);

    // 13. Check that CIDs pinned after sync does not contain
    //     CIDs removed in step #10
    lsResult = await ipfsInstance.getApi().pin.ls();

    const hashesAfterResume = lsResult.map(item => item.hash);

    expect(hashesAfterResume).to.not.contain.members(pinnedHashes);
  });

  after('cleanup lone services', async () => {
    const promises = Promise.all([
      mongoDbInstance.remove(),
      dashCoreInstance.remove(),
      fullDashDriveInstance.remove(),
      dashDriveStandaloneInstance.remove(),
      ipfsInstance.remove(),
    ]);
    await promises;
  });
});
