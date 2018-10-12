const getStateTransitionPackets = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const StateTransitionPacket = require('../../../lib/storage/StateTransitionPacket');

const ApiAppOptions = require('../../../lib/app/ApiAppOptions');

const registerUser = require('../../../lib/test/registerUser');
const createSTHeader = require('../../../lib/test/createSTHeader');

const { startDashDrive } = require('@dashevo/js-evo-services-ctl');

const wait = require('../../../lib/util/wait');
const cbor = require('cbor');

const apiAppOptions = new ApiAppOptions(process.env);

async function createAndSubmitST(userId, privateKeyString, username, basePacketData, instance) {
  const packet = new StateTransitionPacket(basePacketData);
  packet.dapcontract.description = `Valid registration for ${username}`;

  const header = await createSTHeader(userId, privateKeyString, packet);

  const serializedPacket = cbor.encodeCanonical(packet.toJSON({ skipMeta: true }));
  const serializedPacketJson = {
    packet: serializedPacket.toString('hex'),
  };
  const { result: packetCid } = await instance.driveApi.getApi()
    .request('addSTPacket', serializedPacketJson);

  const { result: tsid } = await instance.dashCore.getApi().sendRawTransaction(header);
  await instance.dashCore.getApi().generate(1);

  return { packetCid, tsid };
}

async function blockCountEvenAndEqual(
  instanceOne,
  instanceTwo,
  desiredBlockCount = -1,
  timeout = 90,
) {
  for (let i = 0; i < timeout; i++) {
    const { result: blockCountOne } = await instanceOne.getApi().getBlockCount();
    const { result: blockCountTwo } = await instanceTwo.getApi().getBlockCount();

    if (blockCountOne === blockCountTwo) {
      if (blockCountOne === desiredBlockCount) {
        break;
      } else {
        throw new Error(`Block count of ${blockCountOne} is not desirable ${desiredBlockCount}`);
      }
    } else if (i === timeout - 1) {
      throw new Error('Timeout waiting for block count to be equal on both nodes');
    }

    await wait(1000);
  }
}

/**
 * Await Dash Drive instance to finish syncing
 *
 * @param {DriveApi} instance
 * @returns {Promise<void>}
 */
async function dashDriveSyncToFinish(instance) {
  let finished = false;
  while (!finished) {
    try {
      const { result: syncInfo } = await instance.getApi()
        .request('getSyncInfo', []);

      if (syncInfo.status === 'synced') {
        finished = true;
        await wait(apiAppOptions.getSyncStateCheckInterval() * 1000);
      } else {
        await wait(1000);
      }
    } catch (e) {
      await wait(1000);
    }
  }
}

describe('Blockchain reorganization', function main() {
  let firstDashDrive;
  let secondDashDrive;

  let packetsCids;
  let packetsAddedAfterDisconnect;
  let stPackets;
  let transitionsAfterDisconnect;

  let registeredUsers;

  let initialSyncBeforeReconnectAt;

  const BLOCKS_PER_ST = 1;
  const BLOCKS_PER_REGISTRATION = 108;
  const BLOCKS_PROPAGATION_ACTIVATION = 1;
  const BLOCKS_ST_ACTIVATION = 1000;

  this.timeout(900000);

  before('having started Dash Drive node and generated some STs', async () => {
    packetsCids = [];
    packetsAddedAfterDisconnect = [];
    transitionsAfterDisconnect = [];
    registeredUsers = [];

    stPackets = getStateTransitionPackets();

    // 1. Start two full Dash Drive instances
    [firstDashDrive, secondDashDrive] = await startDashDrive.many(2);

    // 1.1 Activate Special Transactions
    await firstDashDrive.dashCore.getApi().generate(BLOCKS_ST_ACTIVATION);

    // Register a pool of users.
    // Do that here so major part of blocks are in the beginning
    for (let i = 0; i < 10; i++) {
      const instance = firstDashDrive;
      const username = `Alice_${i}`;
      const { userId, privateKeyString } =
            await registerUser(username, instance.dashCore.getApi());
      registeredUsers.push({ username, userId, privateKeyString });
    }

    // Await number of blocks even on both nodes
    await blockCountEvenAndEqual(
      firstDashDrive.dashCore,
      secondDashDrive.dashCore,
      BLOCKS_PROPAGATION_ACTIVATION + BLOCKS_ST_ACTIVATION +
      (10 * BLOCKS_PER_REGISTRATION),
    );

    // 2. Populate instance of Dash Drive and Dash Core with data
    //    First two STs, should be equal on both nodes
    for (let i = 0; i < 2; i++) {
      const user = registeredUsers.pop();
      const { packetCid } = await createAndSubmitST(
        user.userId,
        user.privateKeyString,
        user.username,
        stPackets[0],
        firstDashDrive,
      );
      packetsCids.push(packetCid);
    }

    // 3. Await block count to be equal on both nodes
    //    Should be equal number of generated STs times number of blocks per ST
    await blockCountEvenAndEqual(
      firstDashDrive.dashCore,
      secondDashDrive.dashCore,
      BLOCKS_PROPAGATION_ACTIVATION + BLOCKS_ST_ACTIVATION +
      (10 * BLOCKS_PER_REGISTRATION) + (2 * BLOCKS_PER_ST),
    );

    // Await first Dash Drive sync
    await dashDriveSyncToFinish(firstDashDrive.driveApi);
  });

  it('Dash Drive should sync data after blockchain reorganization, removing missing STs. Adding them back after they reappear in the blockchain.', async () => {
    // 4. Disconnecting nodes to start introducing difference in blocks
    firstDashDrive.dashCore.disconnect(secondDashDrive.dashCore);

    // 5. Generate two more ST on the first node
    //    Note: keep track of exact those CIDs as they should disappear after reorganization
    //    Note: keep track of tsid as well to check if it's moved in mempool later on
    for (let i = 0; i < 2; i++) {
      const user = registeredUsers.pop();
      const { packetCid, tsid } = await createAndSubmitST(
        user.userId,
        user.privateKeyString,
        user.username,
        stPackets[0],
        firstDashDrive,
      );
      packetsCids.push(packetCid);
      packetsAddedAfterDisconnect.push(packetCid);
      transitionsAfterDisconnect.push(tsid);
    }

    // Check tses are not in mempool
    const { result: tsIdsAfterDisconnect } = await firstDashDrive.dashCore.getApi().getRawMemPool();
    for (let i = 0; i < transitionsAfterDisconnect.length - 1; i++) {
      const tsid = transitionsAfterDisconnect[i];
      expect(tsIdsAfterDisconnect).to.not.include(tsid);
    }

    // 6. Check proper block count on the first node
    {
      const { result: blockCount } = await firstDashDrive.dashCore.getApi().getBlockCount();

      const expectedBlockCount = BLOCKS_PROPAGATION_ACTIVATION +
                                 BLOCKS_ST_ACTIVATION +
                                 (10 * BLOCKS_PER_REGISTRATION) + (4 * BLOCKS_PER_ST);

      expect(blockCount).to.be.equal(expectedBlockCount);
    }

    // 7. Generate slightly larger amount of STs on the second node
    //    to introduce reorganization
    for (let i = 0; i < 3; i++) {
      const user = registeredUsers.pop();
      const { packetCid } = await createAndSubmitST(
        user.userId,
        user.privateKeyString,
        user.username,
        stPackets[0],
        secondDashDrive,
      );
      packetsCids.push(packetCid);
    }

    // 8. Check proper block count on the second node
    {
      const { result: blockCount } = await secondDashDrive.dashCore.getApi().getBlockCount();

      const expectedBlockCount = BLOCKS_PROPAGATION_ACTIVATION +
                                 BLOCKS_ST_ACTIVATION +
                                 (10 * BLOCKS_PER_REGISTRATION) + (5 * BLOCKS_PER_ST);

      expect(blockCount).to.be.equal(expectedBlockCount);
    }

    // Await for Drive to sync
    await dashDriveSyncToFinish(firstDashDrive.driveApi);

    // Store `lastInitialSyncAt` to check later that no
    // initial sync happened after reconnect
    ({ result: { lastInitialSyncAt: initialSyncBeforeReconnectAt } } = await firstDashDrive
      .driveApi.getApi()
      .request('getSyncInfo', []));

    // 9. Reconnect nodes
    await firstDashDrive.dashCore.connect(secondDashDrive.dashCore);

    // 10. Await equal block count on both nodes
    //     Notes: should be equal to largest chain
    await blockCountEvenAndEqual(
      firstDashDrive.dashCore,
      secondDashDrive.dashCore,
      BLOCKS_PROPAGATION_ACTIVATION + BLOCKS_ST_ACTIVATION +
      (10 * BLOCKS_PER_REGISTRATION) + (5 * BLOCKS_PER_ST),
    );

    // Check tses are back to mempool
    const { result: tsIdsAfterConnect } = await firstDashDrive.dashCore.getApi().getRawMemPool();
    for (let i = 0; i < transitionsAfterDisconnect.length - 1; i++) {
      const tsid = transitionsAfterDisconnect[i];
      expect(tsIdsAfterConnect).to.include(tsid);
    }

    // 11. Await Dash Drive to sync
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    // 12. Check packet CIDs added after disconnect does not appear in Dash Drive
    {
      const lsResult = await secondDashDrive.ipfs.getApi().pin.ls();
      const lsHashes = lsResult.map(item => item.hash);

      packetsAddedAfterDisconnect.forEach((cid) => {
        expect(lsHashes).to.not.include(cid);
      });
    }

    {
      const lsResult = await firstDashDrive.ipfs.getApi().pin.ls();
      const lsHashes = lsResult.map(item => item.hash);

      packetsAddedAfterDisconnect.forEach((cid) => {
        expect(lsHashes).to.not.include(cid);
      });

      // Check `lastInitialSyncAt is not changed
      // This will indicate no initial sync happened after reconnect
      const { result: { lastInitialSyncAt } } = await firstDashDrive.driveApi.getApi()
        .request('getSyncInfo', []);

      expect(lastInitialSyncAt).to.be.equal(initialSyncBeforeReconnectAt);
    }

    // 13. Generate more blocks so TSes reappear on the blockchain
    await firstDashDrive.dashCore.getApi().generate(10);

    // 14. Await Dash Drive to sync
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    // 15. Check CIDs reappear in Dash Drive
    {
      const lsResult = await secondDashDrive.ipfs.getApi().pin.ls();
      const lsHashes = lsResult.map(item => item.hash);

      packetsCids.forEach((cid) => {
        expect(lsHashes).to.include(cid);
      });
    }

    {
      const lsResult = await firstDashDrive.ipfs.getApi().pin.ls();
      const lsHashes = lsResult.map(item => item.hash);

      packetsAddedAfterDisconnect.forEach((cid) => {
        expect(lsHashes).to.include(cid);
      });
    }
  });

  after('cleanup lone services', async () => {
    const instances = [
      firstDashDrive,
      secondDashDrive,
    ];

    await Promise.all(instances.filter(i => i)
      .map(i => i.remove()));
  });
});
