const DashPlatformProtocol = require('@dashevo/dpp');

const { startDashDrive } = require('@dashevo/js-evo-services-ctl');

const ApiAppOptions = require('../../../lib/app/ApiAppOptions');

const registerUser = require('../../../lib/test/registerUser');

const createStateTransition = require('../../../lib/test/createStateTransition');
const wait = require('../../../lib/util/wait');


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
        await wait(apiAppOptions.getSyncStateCheckInterval());
      } else {
        await wait(1000);
      }
    } catch (e) {
      await wait(1000);
    }
  }
}

async function sendSTPacket(
  userId,
  privateKeyString,
  username,
  stPacket,
  instance,
  previousTransitionHash = undefined,
) {
  const stateTransition = await createStateTransition(
    userId,
    privateKeyString,
    stPacket,
    previousTransitionHash,
  );

  const params = {
    packet: stPacket.serialize().toString('hex'),
  };

  const { error } = await instance.driveApi.getApi().request('addSTPacket', params);

  if (error) {
    throw new Error(`Can't add ST Packet: ${JSON.stringify(error)}`);
  }

  const { result: tsId } = await instance.dashCore.getApi().sendRawTransaction(stateTransition);

  await instance.dashCore.getApi().generate(1);

  return { tsId };
}

describe('Initial sync of Dash Drive and Dash Core', function main() {
  let firstDashDrive;
  let secondDashDrive;
  let users;
  let dpp;
  let dpContract;
  let objectType;

  this.timeout(900000);

  before('having Dash Drive node #1 up and ready, some amount of STs generated and Dash Drive on node #1 fully synced', async () => {
    dpp = new DashPlatformProtocol();

    // 1. Start first Dash Drive node
    firstDashDrive = await startDashDrive();

    // 1.1 Activate Special Transactions
    await firstDashDrive.dashCore.getApi().generate(1000);

    // 2. Register a bunch of users on a blockchain
    users = [];

    for (let i = 0; i < 4; i++) {
      const user = {
        username: `BC_USER_${i}`,
        aboutMe: `User ${i} description`,
      };

      ({
        userId: user.userId,
        privateKeyString: user.privateKeyString,
      } = await registerUser(
        user.username,
        firstDashDrive.dashCore.getApi(),
      ));

      users.push(user);
    }

    // 3. Create DP Contract
    objectType = 'user';
    dpContract = dpp.contract.create('TestContract', {
      [objectType]: {
        properties: {
          aboutMe: {
            type: 'string',
          },
        },
        additionalProperties: false,
      },
    });

    dpp.setDPContract(dpContract);

    const dpContractPacket = dpp.packet.create(dpContract);

    const { tsId: dpContractTsId } = await sendSTPacket(
      users[0].userId,
      users[0].privateKeyString,
      users[0].username,
      dpContractPacket,
      firstDashDrive,
    );

    // 3.1 Await Drive to sync
    await dashDriveSyncToFinish(firstDashDrive.driveApi);

    // 3.2 Check DP Contract is in Drive and ok
    const { result: rawDPContract } = await firstDashDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: dpContract.getId() });

    expect(rawDPContract).to.be.deep.equal(dpContract.toJSON());

    // 4. Create a bunch of `user` DP Objects (for every blockchain user)
    let prevTransitionId;

    for (let i = 0; i < users.length; i++) {
      const user = users[i];

      // if it's the user used to register contractId, use it
      // use nothing if else
      if (i === 0) {
        prevTransitionId = dpContractTsId;
      } else {
        prevTransitionId = user.userId;
      }

      dpp.setUserId(user.userId);

      const userDPObject = dpp.object.create(objectType, {
        aboutMe: user.aboutMe,
      });

      const stPacket = dpp.packet.create([userDPObject]);

      ({ tsId: user.prevTransitionId } = await sendSTPacket(
        user.userId,
        user.privateKeyString,
        user.username,
        stPacket,
        firstDashDrive,
        prevTransitionId,
      ));
    }
  });

  it('Dash Drive should sync the data with Dash Core upon startup', async () => {
    // 3. Start 2nd Dash Drive node and connect to the first one
    secondDashDrive = await startDashDrive();
    await secondDashDrive.ipfs.connect(firstDashDrive.ipfs);
    await secondDashDrive.dashCore.connect(firstDashDrive.dashCore);

    // 4. Await Dash Drive on the 2nd node to finish syncing
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    // 5. Ensure second Dash Drive have a proper data
    const driveApi = secondDashDrive.driveApi.getApi();

    const { result: fetchedDPContract } = await driveApi.request('fetchDPContract', {
      contractId: dpContract.getId(),
    });

    expect(fetchedDPContract).to.be.deep.equal(dpContract.toJSON());

    const { result: fetchedDPObjects } = await driveApi.request('fetchDPObjects', {
      contractId: dpContract.getId(),
      type: objectType,
    });

    expect(fetchedDPObjects).to.have.lengthOf(users.length);

    const aboutMes = fetchedDPObjects.map(o => o.aboutMe);

    for (let i = 0; i < users.length; i++) {
      expect(aboutMes).to.include(users[i].aboutMe);
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
