const addSTPacketFactory = require('../../../lib/storage/ipfs/addSTPacketFactory');
const getStateTransitionPackets = require('../../../lib/test/fixtures/getTransitionPacketFixtures');

const registerUser = require('../../../lib/test/registerUser');
const createSTHeader = require('../../../lib/test/createSTHeader');

const { startDashDrive } = require('js-evo-services-ctl');

const wait = require('../../../lib/util/wait');
const cbor = require('cbor');

/**
 * Await Dash Drive instance to finish syncing
 *
 * @param {DriveApi} instance
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
  let firstDashDrive;
  let secondDashDrive;

  let packetsCids;
  let packetsData;

  this.timeout(900000);

  before('having Dash Drive node #1 up and ready, some amount of STs generated and Dash Drive on node #1 fully synced', async () => {
    packetsCids = [];
    packetsData = getStateTransitionPackets();

    // 1. Start first Dash Drive node
    firstDashDrive = await startDashDrive();

    // 1.1 Activate Special Transactions
    await firstDashDrive.dashCore.getApi().generate(1000);

    // 2. Populate Dash Drive and Dash Core With data
    async function createAndSubmitST(username) {
      // 2.1 Get packet data with random object description
      const packetOne = packetsData[0];
      packetOne.dapcontract.description = `Valid registration for ${username}`;

      // 2.2 Register user and create DAP Contract State Transition packet and header
      const { userId, privateKeyString } =
        await registerUser(username, firstDashDrive.dashCore.getApi());
      const header = await createSTHeader(userId, privateKeyString, packetOne);

      // 2.3 Add ST packet to IPFS
      const addSTPacket = addSTPacketFactory(firstDashDrive.ipfs.getApi());
      const packetCid = await addSTPacket(packetOne);

      // 2.4 Save CID of freshly added packet for future use
      packetsCids.push(packetCid);

      // 2.5 Send ST header to Dash Core and generate a block with it
      await firstDashDrive.dashCore.getApi().sendRawTransition(header.serialize());
      await firstDashDrive.dashCore.getApi().generate(1);
    }

    // Note: I can't use Promise.all here due to errors with PrivateKey
    //       I guess some of the actions can't be executed in parallel
    for (let i = 0; i < 4; i++) {
      await createAndSubmitST(`Alice_${i}`);
    }
  });

  it('Dash Drive should sync the data with Dash Core upon startup', async () => {
    // 3. Start 2nd Dash Drive node and connect to the first one
    secondDashDrive = await startDashDrive();
    await secondDashDrive.ipfs.connect(firstDashDrive.ipfs);
    await secondDashDrive.dashCore.connect(firstDashDrive.dashCore);

    // 4. Await Dash Drive on the 2nd node to finish syncing
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    // 5. Get all pinned CIDs on the 2nd node and assert
    //    they contain CIDs saved from the 1st node
    const lsResult = await secondDashDrive.ipfs.getApi().pin.ls();

    const hashes = lsResult.map(item => item.hash);

    expect(hashes).to.contain.members(packetsCids);
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
