const addSTPacketFactory = require('../../../lib/storage/ipfs/addSTPacketFactory');
const getStateTransitionPackets = require('../../../lib/test/fixtures/getTransitionPacketFixtures');

const ApiAppOptions = require('../../../lib/app/ApiAppOptions');

const StateTransitionPacket = require('../../../lib/storage/StateTransitionPacket');

const registerUser = require('../../../lib/test/registerUser');
const createSTHeader = require('../../../lib/test/createSTHeader');

const { startDashDrive } = require('@dashevo/js-evo-services-ctl');

const wait = require('../../../lib/util/wait');
const cbor = require('cbor');

const apiAppOptions = new ApiAppOptions(process.env);

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

async function createAndSubmitST(
  userId,
  privateKeyString,
  username,
  basePacketData,
  instance,
  previousTransitionHash = undefined,
) {
  const packet = new StateTransitionPacket(basePacketData);

  const header = await createSTHeader(userId, privateKeyString, packet, previousTransitionHash);

  const addSTPacket = addSTPacketFactory(instance.ipfs.getApi());
  const packetCid = await addSTPacket(packet);

  const { result: tsId } = await instance.dashCore.getApi().sendRawTransaction(header);
  await instance.dashCore.getApi().generate(1);

  return { packetCid, tsId };
}

describe('Initial sync of Dash Drive and Dash Core', function main() {
  let firstDashDrive;
  let secondDashDrive;

  let packetsCids;
  let packetsData;

  let users;

  let dapId;

  this.timeout(900000);

  before('having Dash Drive node #1 up and ready, some amount of STs generated and Dash Drive on node #1 fully synced', async () => {
    packetsCids = [];
    packetsData = getStateTransitionPackets();
    users = [];

    // 1. Start first Dash Drive node
    firstDashDrive = await startDashDrive();

    // 1.1 Activate Special Transactions
    await firstDashDrive.dashCore.getApi().generate(1000);

    // 2. Register a bunch of users on a blockchain
    for (let i = 0; i < 4; i++) {
      const user = {
        username: `BC_USER_${i}`,
        aboutMe: `Something about BC_USER_${i}`,
      };

      ({ userId: user.userId, privateKeyString: user.privateKeyString } =
       await registerUser(user.username, firstDashDrive.dashCore.getApi()));

      users.push(user);
    }

    // 3. Create DAP Contract
    let packetCid;
    ({ packetCid, tsId: dapId } = await createAndSubmitST(
      users[0].userId,
      users[0].privateKeyString,
      users[0].username,
      packetsData[0],
      firstDashDrive,
    ));

    packetsCids.push(packetCid);

    // 4. Register a bunch of `user` DAP Objects (for every blockchain user)
    let prevTransitionId;

    for (let i = 0; i < users.length; i++) {
      const user = users[i];

      // if it's the user used to register dapId, use it
      // use nothing if else
      if (i === 0) {
        prevTransitionId = dapId;
      } else {
        prevTransitionId = undefined;
      }

      const userData = Object.assign({}, packetsData[1], {
        dapid: dapId,
        dapobjects: [
          {
            id: i + 1,
            objtype: 'user',
            aboutme: user.aboutMe,
            pver: 1,
            idx: 0,
            rev: 1,
            act: 1,
          },
        ],
      });

      user.userData = userData;

      ({ packetCid, tsId: user.prevTransitionId } = await createAndSubmitST(
        user.userId,
        user.privateKeyString,
        user.username,
        userData,
        firstDashDrive,
        prevTransitionId,
      ));

      packetsCids.push(packetCid);
    }
  });

  it('Dash Drive should sync the data with Dash Core upon startup', async () => {
    // 3. Start 2nd Dash Drive node and connect to the first one
    secondDashDrive = await startDashDrive();
    await secondDashDrive.ipfs.connect(firstDashDrive.ipfs);
    await secondDashDrive.dashCore.connect(firstDashDrive.dashCore);

    // 4. Add ST packet to Drive
    const packet = getStateTransitionPackets()[0];
    const serializedPacket = cbor.encodeCanonical(packet.toJSON({ skipMeta: true }));
    const serializedPacketJson = {
      packet: serializedPacket.toString('hex'),
    };
    await secondDashDrive.driveApi.getApi()
      .request('addSTPacket', serializedPacketJson);

    // 5. Await Dash Drive on the 2nd node to finish syncing
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    // 6. Ensure second Dash Drive have a proper data
    {
      const { result: objects } = await secondDashDrive.driveApi.getApi()
        .request('fetchDapObjects', { dapId, type: 'user' });
      expect(objects.length).to.be.equal(users.length);

      const aboutMes = objects.map(o => o.object.aboutme);

      for (let i = 0; i < users.length; i++) {
        expect(aboutMes).to.include(users[i].aboutMe);
      }
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
