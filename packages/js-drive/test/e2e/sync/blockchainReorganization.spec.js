const { startDrive } = require('@dashevo/dp-services-ctl');
const DashPlatformPlatform = require('@dashevo/dpp');

const ApiAppOptions = require('../../../lib/app/ApiAppOptions');

const registerUser = require('../../../lib/test/registerUser');

const createStateTransition = require('../../../lib/test/createStateTransition');
const wait = require('../../../lib/util/wait');

const apiAppOptions = new ApiAppOptions(process.env);

async function createAndSubmitST(
  userId,
  privateKeyString,
  stPacket,
  instance,
  previousTransitionHash = undefined,
) {
  const stateTransition = await createStateTransition(
    userId, privateKeyString, stPacket, previousTransitionHash,
  );

  const { error } = await instance.driveApi.getApi().request('addSTPacket', {
    stPacket: stPacket.serialize().toString('hex'),
    stateTransition: stateTransition.serialize(),
  });

  if (error) {
    throw new Error(`Can't add ST Packet: ${JSON.stringify(error)}`);
  }

  const { result: txId } = await instance.dashCore.getApi().sendRawTransaction(
    stateTransition.serialize(),
  );

  await instance.dashCore.getApi().generate(1);

  return txId;
}

async function blockCountEvenAndEqual(
  instanceOne,
  instanceTwo,
  timeout = 90,
) {
  for (let i = 0; i < timeout; i++) {
    const { result: blockCountOne } = await instanceOne.getApi().getBlockCount();
    const { result: blockCountTwo } = await instanceTwo.getApi().getBlockCount();

    if (blockCountOne === blockCountTwo) {
      break;
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
async function driveSyncToFinish(instance) {
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

describe('Blockchain reorganization', function main() {
  let firstDrive;
  let secondDrive;

  let firstUser;
  let secondUser;
  let thirdUser;

  let firstContractPacket;
  let secondContractPacket;
  let thirdContractPacket;

  let firstObjectPacket;
  let secondObjectPacket;
  let thirdObjectPacket;

  let objectType;

  const BLOCKS_ST_ACTIVATION = 1000;

  this.timeout(900000);

  before('having started Dash Drive node and generated some STs', async () => {
    const dpp = new DashPlatformPlatform();

    objectType = 'user';

    const contractPackets = [];
    for (let i = 1; i <= 3; i++) {
      const dpContract = dpp.contract.create(`contract${i}`, {
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

      contractPackets.push(
        dpp.packet.create(dpContract),
      );
    }
    [firstContractPacket, secondContractPacket, thirdContractPacket] = contractPackets;

    // Start two full Dash Drive instances
    [firstDrive, secondDrive] = await startDrive.many(2);

    // Activate Special Transactions
    await firstDrive.dashCore.getApi().generate(BLOCKS_ST_ACTIVATION);

    // Register a pool of users.
    // Do that here so major part of blocks are in the beginning
    const registeredUsers = [];
    for (let i = 1; i <= 3; i++) {
      const username = `user${i}`;

      const { userId, privateKeyString } = await registerUser(
        username,
        firstDrive.dashCore.getApi(),
      );

      registeredUsers.push({ username, userId, privateKeyString });
    }

    [firstUser, secondUser, thirdUser] = registeredUsers;

    // Mine block with SubTx + 6 blocks on top of it
    await firstDrive.dashCore.getApi().generate(7);

    // Await number of blocks even on both nodes
    await blockCountEvenAndEqual(
      firstDrive.dashCore,
      secondDrive.dashCore,
    );

    // Register first contract
    const firstContractTxId = await createAndSubmitST(
      firstUser.userId,
      firstUser.privateKeyString,
      firstContractPacket,
      firstDrive,
    );

    // Sync first contract
    await firstDrive.dashCore.getApi().generate(1);
    await driveSyncToFinish(firstDrive.driveApi);

    dpp.setUserId(firstUser.userId);
    dpp.setDPContract(firstContractPacket.getDPContract());

    const firstObject = dpp.object.create(objectType, {
      aboutMe: 'About first user',
    });

    firstObjectPacket = dpp.packet.create([firstObject]);

    // Register first object
    await createAndSubmitST(
      firstUser.userId,
      firstUser.privateKeyString,
      firstObjectPacket,
      firstDrive,
      firstContractTxId,
    );

    // Await block count to be equal on both nodes
    await blockCountEvenAndEqual(
      firstDrive.dashCore,
      secondDrive.dashCore,
    );

    // Await Drive nodes to be in sync with Core
    await driveSyncToFinish(firstDrive.driveApi);
    await driveSyncToFinish(secondDrive.driveApi);

    // Check data is on both Drive nodes
    // Check data on first node
    const { result: firstDriveFirstContract } = await firstDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: firstContractPacket.getDPContractId() });

    const { result: [firstDriveFirstObject] } = await firstDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: firstContractPacket.getDPContractId(), type: objectType });

    expect(firstDriveFirstContract).to.deep.equal(firstContractPacket.getDPContract().toJSON());
    expect(firstDriveFirstObject).to.deep.equal(firstObjectPacket.getDPObjects()[0].toJSON());

    // Check data on the second node
    const { result: secondDriveFirstContract } = await secondDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: firstContractPacket.getDPContractId() });

    const { result: [secondDriveFirstObject] } = await secondDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: firstContractPacket.getDPContractId(), type: objectType });

    expect(secondDriveFirstContract).to.deep.equal(firstContractPacket.getDPContract().toJSON());
    expect(secondDriveFirstObject).to.deep.equal(firstObjectPacket.getDPObjects()[0].toJSON());

    // Disconnect Core nodes
    await firstDrive.dashCore.disconnect(secondDrive.dashCore);

    // Generate 2nd contract and object on the first Drive node
    const secondContractTxId = await createAndSubmitST(
      secondUser.userId,
      secondUser.privateKeyString,
      secondContractPacket,
      firstDrive,
    );

    // Sync second contract
    await firstDrive.dashCore.getApi().generate(1);
    await driveSyncToFinish(firstDrive.driveApi);

    dpp.setUserId(secondUser.userId);
    dpp.setDPContract(secondContractPacket.getDPContract());

    const secondObject = dpp.object.create(objectType, {
      aboutMe: 'About second user',
    });

    secondObjectPacket = dpp.packet.create([secondObject]);

    // Register an object
    await createAndSubmitST(
      secondUser.userId,
      secondUser.privateKeyString,
      secondObjectPacket,
      firstDrive,
      secondContractTxId,
    );

    await driveSyncToFinish(firstDrive.driveApi);

    // Check second contract and object is created on the first node
    const { result: firstDriveSecondContract } = await firstDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: secondContractPacket.getDPContractId() });

    const { result: [firstDriveSecondObject] } = await firstDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: secondContractPacket.getDPContractId(), type: objectType });

    expect(firstDriveSecondContract).to.deep.equal(
      secondContractPacket.getDPContract().toJSON(),
    );

    expect(firstDriveSecondObject).to.deep.equal(
      secondObjectPacket.getDPObjects()[0].toJSON(),
    );

    // Generate 2 more blocks, 3rd contract and object on the second Drive node
    // To introduce a slightly bigger fork
    await secondDrive.dashCore.getApi().generate(1);

    const thirdContractTxId = await createAndSubmitST(
      thirdUser.userId,
      thirdUser.privateKeyString,
      thirdContractPacket,
      secondDrive,
    );

    // Sync third contract
    await secondDrive.dashCore.getApi().generate(1);
    await driveSyncToFinish(firstDrive.driveApi);

    dpp.setUserId(thirdUser.userId);
    dpp.setDPContract(thirdContractPacket.getDPContract());

    const thirdObject = dpp.object.create(objectType, {
      aboutMe: 'About third user',
    });

    thirdObjectPacket = dpp.packet.create([thirdObject]);

    // Register an object
    await createAndSubmitST(
      thirdUser.userId,
      thirdUser.privateKeyString,
      thirdObjectPacket,
      secondDrive,
      thirdContractTxId,
    );

    await driveSyncToFinish(secondDrive.driveApi);

    // Check third contract and object are created on the second node
    const { result: secondDriveThirdContract } = await secondDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: thirdContractPacket.getDPContractId() });

    const { result: [secondDriveThirdObject] } = await secondDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: thirdContractPacket.getDPContractId(), type: objectType });

    expect(secondDriveThirdContract).to.deep.equal(
      thirdContractPacket.getDPContract().toJSON(),
    );

    expect(secondDriveThirdObject).to.deep.equal(
      thirdObjectPacket.getDPObjects()[0].toJSON(),
    );
  });

  it('Dash Drive should sync data after blockchain reorganization, removing missing STs. Adding them back after they reappear on the blockchain.', async () => {
    // Save initialSyncAt to test it later and make sure
    // There was no initial sync
    const {
      result: {
        lastInitialSyncAt: lastInitialSyncAtBefore,
      },
    } = await firstDrive.driveApi.getApi().request('getSyncInfo', []);

    // Reconnect both Core nodes
    await firstDrive.dashCore.connect(secondDrive.dashCore);

    // Await block count to be equal on both nodes
    await blockCountEvenAndEqual(
      firstDrive.dashCore,
      secondDrive.dashCore,
    );

    // Await Drive nodes to be in sync with Core
    await driveSyncToFinish(firstDrive.driveApi);
    await driveSyncToFinish(secondDrive.driveApi);

    //
    // Check first contract and object are in place on both nodes
    //
    // Check the first node
    const { result: firstDriveFirstContract } = await firstDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: firstContractPacket.getDPContractId() });

    const { result: [firstDriveFirstObject] } = await firstDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: firstContractPacket.getDPContractId(), type: objectType });

    expect(firstDriveFirstContract).to.deep.equal(
      firstContractPacket.getDPContract().toJSON(),
    );
    expect(firstDriveFirstObject).to.deep.equal(
      firstObjectPacket.getDPObjects()[0].toJSON(),
    );

    // Check the second node
    const { result: secondDriveFirstContract } = await secondDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: firstContractPacket.getDPContractId() });

    const { result: [secondDriveFirstObject] } = await secondDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: firstContractPacket.getDPContractId(), type: objectType });

    expect(secondDriveFirstContract).to.deep.equal(
      firstContractPacket.getDPContract().toJSON(),
    );
    expect(secondDriveFirstObject).to.deep.equal(
      firstObjectPacket.getDPObjects()[0].toJSON(),
    );

    //
    // Check third contract is on the both nodes now
    //
    // Check the first node
    const { result: firstDriveThirdContract } = await firstDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: thirdContractPacket.getDPContractId() });

    const { result: [firstDriveThirdObject] } = await firstDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: thirdContractPacket.getDPContractId(), type: objectType });

    expect(firstDriveThirdContract).to.deep.equal(
      thirdContractPacket.getDPContract().toJSON(),
    );
    expect(firstDriveThirdObject).to.deep.equal(
      thirdObjectPacket.getDPObjects()[0].toJSON(),
    );

    // Check the second node
    const { result: secondDriveThirdContract } = await secondDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: thirdContractPacket.getDPContractId() });

    const { result: [secondDriveThirdObject] } = await secondDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: thirdContractPacket.getDPContractId(), type: objectType });

    expect(secondDriveThirdContract).to.deep.equal(
      thirdContractPacket.getDPContract().toJSON(),
    );
    expect(secondDriveThirdObject).to.deep.equal(
      thirdObjectPacket.getDPObjects()[0].toJSON(),
    );

    //
    // Check second contract and object are gone from the first Drive node
    // and they are not on the second node as well
    //
    // Check the first node
    const { result: firstDriveSecondContract } = await firstDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: secondContractPacket.getDPContractId() });

    const { result: [firstDriveSecondObject] } = await firstDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: secondContractPacket.getDPContractId(), type: objectType });

    expect(firstDriveSecondContract).to.be.undefined();
    expect(firstDriveSecondObject).to.be.undefined();

    // Check the second node
    const { result: secondDriveSecondContract } = await secondDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: secondContractPacket.getDPContractId() });

    const { result: [secondDriveSecondObject] } = await secondDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: secondContractPacket.getDPContractId(), type: objectType });

    expect(secondDriveSecondContract).to.be.undefined();
    expect(secondDriveSecondObject).to.be.undefined();

    // Generate more blocks so transitions are back from mempool
    await firstDrive.dashCore.getApi().generate(5);

    // Await block count to be equal on both nodes
    await blockCountEvenAndEqual(
      firstDrive.dashCore,
      secondDrive.dashCore,
    );

    // Await Drive nodes to be in sync with Core
    await driveSyncToFinish(firstDrive.driveApi);
    await driveSyncToFinish(secondDrive.driveApi);

    //
    // Check data is back from the mempool after generating more blocks
    // On both nodes
    //
    // Check the first node
    const { result: firstDriveSecondContractAfter } = await firstDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: secondContractPacket.getDPContractId() });

    const { result: [firstDriveSecondObjectAfter] } = await firstDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: secondContractPacket.getDPContractId(), type: objectType });

    expect(firstDriveSecondContractAfter).to.deep.equal(
      secondContractPacket.getDPContract().toJSON(),
    );
    expect(firstDriveSecondObjectAfter).to.deep.equal(
      secondObjectPacket.getDPObjects()[0].toJSON(),
    );

    // Check the second node
    const { result: secondDriveSecondContractAfter } = await secondDrive.driveApi.getApi()
      .request('fetchDPContract', { contractId: secondContractPacket.getDPContractId() });

    const { result: [secondDriveSecondObjectAfter] } = await secondDrive.driveApi.getApi()
      .request('fetchDPObjects', { contractId: secondContractPacket.getDPContractId(), type: objectType });

    expect(secondDriveSecondContractAfter).to.deep.equal(
      secondContractPacket.getDPContract().toJSON(),
    );
    expect(secondDriveSecondObjectAfter).to.deep.equal(
      secondObjectPacket.getDPObjects()[0].toJSON(),
    );

    //
    // Check there was no initial sync
    //
    const {
      result: {
        lastInitialSyncAt: lastInitialSyncAtAfter,
      },
    } = await firstDrive.driveApi.getApi().request('getSyncInfo', []);

    expect(lastInitialSyncAtBefore).to.equal(lastInitialSyncAtAfter);
  });

  after('cleanup services', async () => {
    const instances = [
      firstDrive,
      secondDrive,
    ];

    await Promise.all(instances.filter(i => i)
      .map(i => i.remove()));
  });
});
