const cbor = require('cbor');
const { startDashDrive } = require('@dashevo/js-evo-services-ctl');

const getStateTransitionPackets = require('../../../lib/test/fixtures/getTransitionPacketFixtures');
const StateTransitionPacket = require('../../../lib/storage/StateTransitionPacket');

const ApiAppOptions = require('../../../lib/app/ApiAppOptions');

const registerUser = require('../../../lib/test/registerUser');

const createSTHeader = require('../../../lib/test/createSTHeader');
const wait = require('../../../lib/util/wait');

const doubleSha256 = require('../../../lib/util/doubleSha256');

const apiAppOptions = new ApiAppOptions(process.env);

async function createAndSubmitST(
  userId,
  privateKeyString,
  basePacketData,
  instance,
  previousTransitionHash = undefined,
) {
  const packet = new StateTransitionPacket(basePacketData);

  const header = await createSTHeader(
    userId, privateKeyString, packet, previousTransitionHash,
  );

  const serializedPacket = cbor.encodeCanonical(packet.toJSON({ skipMeta: true }));
  const serializedPacketJson = {
    packet: serializedPacket.toString('hex'),
  };
  await instance.driveApi.getApi()
    .request('addSTPacket', serializedPacketJson);

  const { result: txId } = await instance.dashCore.getApi().sendRawTransaction(header);
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

describe('Blockchain reorganization', function main() {
  let firstDashDrive;
  let secondDashDrive;

  let firstUser;
  let secondUser;
  let thirdUser;

  let firstContractPacket;
  let secondContractPacket;
  let thirdContractPacket;

  let firstObjectPacket;
  let secondObjectPacket;
  let thirdObjectPacket;

  let firstDapId;
  let secondDapId;
  let thirdDapId;

  const BLOCKS_ST_ACTIVATION = 1000;

  this.timeout(900000);

  before('having started Dash Drive node and generated some STs', async () => {
    const [baseContractPacket, baseObjectPacket] = getStateTransitionPackets();

    const contractPackets = [];
    for (let i = 1; i <= 3; i++) {
      const contract = Object.assign({}, baseContractPacket, {
        dapcontract: {
          ...baseContractPacket.dapcontract,
          dapname: `Contract #${i}`,
        },
      });
      contractPackets.push(contract);
    }
    [firstContractPacket, secondContractPacket, thirdContractPacket] = contractPackets;
    [firstDapId, secondDapId, thirdDapId] = contractPackets
      .map(packet => doubleSha256(packet.dapcontract));

    // Start two full Dash Drive instances
    [firstDashDrive, secondDashDrive] = await startDashDrive.many(2);

    // Activate Special Transactions
    await firstDashDrive.dashCore.getApi().generate(BLOCKS_ST_ACTIVATION);

    // Register a pool of users.
    // Do that here so major part of blocks are in the beginning
    const registeredUsers = [];
    for (let i = 1; i <= 3; i++) {
      const instance = firstDashDrive;
      const username = `User #${i}`;
      const { userId, privateKeyString } = await registerUser(username, instance.dashCore.getApi());
      registeredUsers.push({ username, userId, privateKeyString });
    }
    [firstUser, secondUser, thirdUser] = registeredUsers;

    // Await number of blocks even on both nodes
    await blockCountEvenAndEqual(
      firstDashDrive.dashCore,
      secondDashDrive.dashCore,
    );

    // Register first contract
    const firstContractTxId = await createAndSubmitST(
      firstUser.userId,
      firstUser.privateKeyString,
      firstContractPacket,
      firstDashDrive,
    );

    firstObjectPacket = Object.assign({}, baseObjectPacket, {
      dapid: firstDapId,
    });

    // Register first object
    await createAndSubmitST(
      firstUser.userId,
      firstUser.privateKeyString,
      firstObjectPacket,
      firstDashDrive,
      firstContractTxId,
    );

    // Await block count to be equal on both nodes
    await blockCountEvenAndEqual(
      firstDashDrive.dashCore,
      secondDashDrive.dashCore,
    );

    // Await Drive nodes to be in sync with Core
    await dashDriveSyncToFinish(firstDashDrive.driveApi);
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    // Check data is on both Drive nodes
    // Check data on first node
    const { result: firstDriveFirstContract } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: firstDapId });

    const { result: [firstDriveFirstObject] } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: firstDapId, type: 'user' });

    expect(firstDriveFirstContract).to.be.deep.equal(firstContractPacket.dapcontract);
    expect(firstDriveFirstObject).to.be.deep.equal(firstObjectPacket.dapobjects[0]);

    // Check data on the second node
    const { result: secondDriveFirstContract } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: firstDapId });

    const { result: [secondDriveFirstObject] } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: firstDapId, type: 'user' });

    expect(secondDriveFirstContract).to.be.deep.equal(firstContractPacket.dapcontract);
    expect(secondDriveFirstObject).to.be.deep.equal(firstObjectPacket.dapobjects[0]);

    // Disconnect Core nodes
    await firstDashDrive.dashCore.disconnect(secondDashDrive.dashCore);

    // Generate 2nd contract and object on the first Drive node
    const secondContractTxId = await createAndSubmitST(
      secondUser.userId,
      secondUser.privateKeyString,
      secondContractPacket,
      firstDashDrive,
    );

    secondObjectPacket = Object.assign({}, baseObjectPacket, {
      dapid: secondDapId,
    });

    // Register an object
    await createAndSubmitST(
      secondUser.userId,
      secondUser.privateKeyString,
      secondObjectPacket,
      firstDashDrive,
      secondContractTxId,
    );

    await dashDriveSyncToFinish(firstDashDrive.driveApi);

    // Check second contract and object is created on the first node
    const { result: firstDriveSecondContract } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: secondDapId });

    const { result: [firstDriveSecondObject] } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: secondDapId, type: 'user' });

    expect(firstDriveSecondContract).to.be.deep.equal(secondContractPacket.dapcontract);
    expect(firstDriveSecondObject).to.be.deep.equal(secondObjectPacket.dapobjects[0]);

    // Generate 2 more blocks, 3rd contract and object on the second Drive node
    // To introduce a slightly bigger fork
    await secondDashDrive.dashCore.getApi().generate(2);

    const thirdContractTxId = await createAndSubmitST(
      thirdUser.userId,
      thirdUser.privateKeyString,
      thirdContractPacket,
      secondDashDrive,
    );

    thirdObjectPacket = Object.assign({}, baseObjectPacket, {
      dapid: thirdDapId,
    });

    // Register an object
    await createAndSubmitST(
      thirdUser.userId,
      thirdUser.privateKeyString,
      thirdObjectPacket,
      secondDashDrive,
      thirdContractTxId,
    );

    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    // Check third contract and object are created on the second node
    const { result: secondDriveThirdContract } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: thirdDapId });

    const { result: [secondDriveThirdObject] } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: thirdDapId, type: 'user' });

    expect(secondDriveThirdContract).to.be.deep.equal(thirdContractPacket.dapcontract);
    expect(secondDriveThirdObject).to.be.deep.equal(thirdObjectPacket.dapobjects[0]);
  });

  it('Dash Drive should sync data after blockchain reorganization, removing missing STs. Adding them back after they reappear in the blockchain.', async () => {
    // Save initialSyncAt to test it later and make sure
    // There was no intial sync
    const {
      result: {
        lastInitialSyncAt: lastInitialSyncAtBefore,
      },
    } = await firstDashDrive.driveApi.getApi().request('getSyncInfo', []);

    // Reconnect both Core nodes
    await firstDashDrive.dashCore.connect(secondDashDrive.dashCore);

    // Await block count to be equal on both nodes
    await blockCountEvenAndEqual(
      firstDashDrive.dashCore,
      secondDashDrive.dashCore,
    );

    // Await Drive nodes to be in sync with Core
    await dashDriveSyncToFinish(firstDashDrive.driveApi);
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    //
    // Check first contract and object are in place on both nodes
    //
    // Check the first node
    const { result: firstDriveFirstContract } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: firstDapId });

    const { result: [firstDriveFirstObject] } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: firstDapId, type: 'user' });

    expect(firstDriveFirstContract).to.be.deep.equal(firstContractPacket.dapcontract);
    expect(firstDriveFirstObject).to.be.deep.equal(firstObjectPacket.dapobjects[0]);

    // Check the second node
    const { result: secondDriveFirstContract } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: firstDapId });

    const { result: [secondDriveFirstObject] } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: firstDapId, type: 'user' });

    expect(secondDriveFirstContract).to.be.deep.equal(firstContractPacket.dapcontract);
    expect(secondDriveFirstObject).to.be.deep.equal(firstObjectPacket.dapobjects[0]);

    //
    // Check third contract is on the both nodes now
    //
    // Check the first node
    const { result: firstDriveThirdContract } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: thirdDapId });

    const { result: [firstDriveThirdObject] } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: thirdDapId, type: 'user' });

    expect(firstDriveThirdContract).to.be.deep.equal(thirdContractPacket.dapcontract);
    expect(firstDriveThirdObject).to.be.deep.equal(thirdObjectPacket.dapobjects[0]);

    // Check the second node
    const { result: secondDriveThirdContract } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: thirdDapId });

    const { result: [secondDriveThirdObject] } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: thirdDapId, type: 'user' });

    expect(secondDriveThirdContract).to.be.deep.equal(thirdContractPacket.dapcontract);
    expect(secondDriveThirdObject).to.be.deep.equal(thirdObjectPacket.dapobjects[0]);

    //
    // Check second contract and object are gone from the first Drive node
    // and they are not on the second node as well
    //
    // Check the first node
    const { result: firstDriveSecondContract } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: secondDapId });

    const { result: [firstDriveSecondObject] } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: secondDapId, type: 'user' });

    expect(firstDriveSecondContract).to.be.undefined();
    expect(firstDriveSecondObject).to.be.undefined();

    // Check the second node
    const { result: secondDriveSecondContract } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: secondDapId });

    const { result: [secondDriveSecondObject] } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: secondDapId, type: 'user' });

    expect(secondDriveSecondContract).to.be.undefined();
    expect(secondDriveSecondObject).to.be.undefined();

    // Generate more blocks so transitions are back from mempool
    await firstDashDrive.dashCore.getApi().generate(5);

    // Await block count to be equal on both nodes
    await blockCountEvenAndEqual(
      firstDashDrive.dashCore,
      secondDashDrive.dashCore,
    );

    // Await Drive nodes to be in sync with Core
    await dashDriveSyncToFinish(firstDashDrive.driveApi);
    await dashDriveSyncToFinish(secondDashDrive.driveApi);

    //
    // Check data is back from the mempool after generating more blocks
    // On both nodes
    //
    // Check the first node
    const { result: firstDriveSecondContractAfter } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: secondDapId });

    const { result: [firstDriveSecondObjectAfter] } = await firstDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: secondDapId, type: 'user' });

    expect(firstDriveSecondContractAfter).to.be.deep.equal(secondContractPacket.dapcontract);
    expect(firstDriveSecondObjectAfter).to.be.deep.equal(secondObjectPacket.dapobjects[0]);

    // Check the second node
    const { result: secondDriveSecondContractAfter } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapContract', { dapId: secondDapId });

    const { result: [secondDriveSecondObjectAfter] } = await secondDashDrive.driveApi.getApi()
      .request('fetchDapObjects', { dapId: secondDapId, type: 'user' });

    expect(secondDriveSecondContractAfter).to.be.deep.equal(secondContractPacket.dapcontract);
    expect(secondDriveSecondObjectAfter).to.be.deep.equal(secondObjectPacket.dapobjects[0]);

    //
    // Check there was no initial sync
    //
    const {
      result: {
        lastInitialSyncAt: lastInitialSyncAtAfter,
      },
    } = await firstDashDrive.driveApi.getApi().request('getSyncInfo', []);

    expect(lastInitialSyncAtBefore).to.be.equal(lastInitialSyncAtAfter);
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
