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

  let firstDocumentPacket;
  let secondDocumentPacket;
  let thirdDocumentPacket;

  let documentType;

  const BLOCKS_ST_ACTIVATION = 1000;

  this.timeout(900000);

  before('having started Dash Drive node and generated some STs', async () => {
    const dpp = new DashPlatformPlatform();

    documentType = 'user';

    const contractPackets = [];
    for (let i = 1; i <= 3; i++) {
      const contract = dpp.contract.create(`contract${i}`, {
        [documentType]: {
          properties: {
            aboutMe: {
              type: 'string',
            },
          },
          additionalProperties: false,
        },
      });

      dpp.setContract(contract);

      contractPackets.push(
        dpp.packet.create(contract),
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
    dpp.setContract(firstContractPacket.getContract());

    const firstDocument = dpp.document.create(documentType, {
      aboutMe: 'About first user',
    });

    firstDocumentPacket = dpp.packet.create([firstDocument]);

    // Register first document
    await createAndSubmitST(
      firstUser.userId,
      firstUser.privateKeyString,
      firstDocumentPacket,
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
      .request('fetchContract', { contractId: firstContractPacket.getContractId() });

    const { result: [firstDriveFirstDocument] } = await firstDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: firstContractPacket.getContractId(), type: documentType });

    expect(firstDriveFirstContract).to.deep.equal(firstContractPacket.getContract().toJSON());
    expect(firstDriveFirstDocument).to.deep.equal(firstDocumentPacket.getDocuments()[0].toJSON());

    // Check data on the second node
    const { result: secondDriveFirstContract } = await secondDrive.driveApi.getApi()
      .request('fetchContract', { contractId: firstContractPacket.getContractId() });

    const { result: [secondDriveFirstDocument] } = await secondDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: firstContractPacket.getContractId(), type: documentType });

    expect(secondDriveFirstContract).to.deep.equal(firstContractPacket.getContract().toJSON());
    expect(secondDriveFirstDocument).to.deep.equal(firstDocumentPacket.getDocuments()[0].toJSON());

    // Disconnect Core nodes
    await firstDrive.dashCore.disconnect(secondDrive.dashCore);

    // Generate 2nd contract and document on the first Drive node
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
    dpp.setContract(secondContractPacket.getContract());

    const secondDocument = dpp.document.create(documentType, {
      aboutMe: 'About second user',
    });

    secondDocumentPacket = dpp.packet.create([secondDocument]);

    // Register an document
    await createAndSubmitST(
      secondUser.userId,
      secondUser.privateKeyString,
      secondDocumentPacket,
      firstDrive,
      secondContractTxId,
    );

    await driveSyncToFinish(firstDrive.driveApi);

    // Check second contract and document is created on the first node
    const { result: firstDriveSecondContract } = await firstDrive.driveApi.getApi()
      .request('fetchContract', { contractId: secondContractPacket.getContractId() });

    const { result: [firstDriveSecondDocument] } = await firstDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: secondContractPacket.getContractId(), type: documentType });

    expect(firstDriveSecondContract).to.deep.equal(
      secondContractPacket.getContract().toJSON(),
    );

    expect(firstDriveSecondDocument).to.deep.equal(
      secondDocumentPacket.getDocuments()[0].toJSON(),
    );

    // Generate 2 more blocks, 3rd contract and document on the second Drive node
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
    dpp.setContract(thirdContractPacket.getContract());

    const thirdDocument = dpp.document.create(documentType, {
      aboutMe: 'About third user',
    });

    thirdDocumentPacket = dpp.packet.create([thirdDocument]);

    // Register an document
    await createAndSubmitST(
      thirdUser.userId,
      thirdUser.privateKeyString,
      thirdDocumentPacket,
      secondDrive,
      thirdContractTxId,
    );

    await driveSyncToFinish(secondDrive.driveApi);

    // Check third contract and document are created on the second node
    const { result: secondDriveThirdContract } = await secondDrive.driveApi.getApi()
      .request('fetchContract', { contractId: thirdContractPacket.getContractId() });

    const { result: [secondDriveThirdDocument] } = await secondDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: thirdContractPacket.getContractId(), type: documentType });

    expect(secondDriveThirdContract).to.deep.equal(
      thirdContractPacket.getContract().toJSON(),
    );

    expect(secondDriveThirdDocument).to.deep.equal(
      thirdDocumentPacket.getDocuments()[0].toJSON(),
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
    // Check first contract and document are in place on both nodes
    //
    // Check the first node
    const { result: firstDriveFirstContract } = await firstDrive.driveApi.getApi()
      .request('fetchContract', { contractId: firstContractPacket.getContractId() });

    const { result: [firstDriveFirstDocument] } = await firstDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: firstContractPacket.getContractId(), type: documentType });

    expect(firstDriveFirstContract).to.deep.equal(
      firstContractPacket.getContract().toJSON(),
    );
    expect(firstDriveFirstDocument).to.deep.equal(
      firstDocumentPacket.getDocuments()[0].toJSON(),
    );

    // Check the second node
    const { result: secondDriveFirstContract } = await secondDrive.driveApi.getApi()
      .request('fetchContract', { contractId: firstContractPacket.getContractId() });

    const { result: [secondDriveFirstDocument] } = await secondDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: firstContractPacket.getContractId(), type: documentType });

    expect(secondDriveFirstContract).to.deep.equal(
      firstContractPacket.getContract().toJSON(),
    );
    expect(secondDriveFirstDocument).to.deep.equal(
      firstDocumentPacket.getDocuments()[0].toJSON(),
    );

    //
    // Check third contract is on the both nodes now
    //
    // Check the first node
    const { result: firstDriveThirdContract } = await firstDrive.driveApi.getApi()
      .request('fetchContract', { contractId: thirdContractPacket.getContractId() });

    const { result: [firstDriveThirdDocument] } = await firstDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: thirdContractPacket.getContractId(), type: documentType });

    expect(firstDriveThirdContract).to.deep.equal(
      thirdContractPacket.getContract().toJSON(),
    );
    expect(firstDriveThirdDocument).to.deep.equal(
      thirdDocumentPacket.getDocuments()[0].toJSON(),
    );

    // Check the second node
    const { result: secondDriveThirdContract } = await secondDrive.driveApi.getApi()
      .request('fetchContract', { contractId: thirdContractPacket.getContractId() });

    const { result: [secondDriveThirdDocument] } = await secondDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: thirdContractPacket.getContractId(), type: documentType });

    expect(secondDriveThirdContract).to.deep.equal(
      thirdContractPacket.getContract().toJSON(),
    );
    expect(secondDriveThirdDocument).to.deep.equal(
      thirdDocumentPacket.getDocuments()[0].toJSON(),
    );

    //
    // Check second contract and document are gone from the first Drive node
    // and they are not on the second node as well
    //
    // Check the first node
    const { result: firstDriveSecondContract } = await firstDrive.driveApi.getApi()
      .request('fetchContract', { contractId: secondContractPacket.getContractId() });

    const { result: [firstDriveSecondDocument] } = await firstDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: secondContractPacket.getContractId(), type: documentType });

    expect(firstDriveSecondContract).to.be.undefined();
    expect(firstDriveSecondDocument).to.be.undefined();

    // Check the second node
    const { result: secondDriveSecondContract } = await secondDrive.driveApi.getApi()
      .request('fetchContract', { contractId: secondContractPacket.getContractId() });

    const { result: [secondDriveSecondDocument] } = await secondDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: secondContractPacket.getContractId(), type: documentType });

    expect(secondDriveSecondContract).to.be.undefined();
    expect(secondDriveSecondDocument).to.be.undefined();

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
      .request('fetchContract', { contractId: secondContractPacket.getContractId() });

    const { result: [firstDriveSecondDocumentAfter] } = await firstDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: secondContractPacket.getContractId(), type: documentType });

    expect(firstDriveSecondContractAfter).to.deep.equal(
      secondContractPacket.getContract().toJSON(),
    );
    expect(firstDriveSecondDocumentAfter).to.deep.equal(
      secondDocumentPacket.getDocuments()[0].toJSON(),
    );

    // Check the second node
    const { result: secondDriveSecondContractAfter } = await secondDrive.driveApi.getApi()
      .request('fetchContract', { contractId: secondContractPacket.getContractId() });

    const { result: [secondDriveSecondDocumentAfter] } = await secondDrive.driveApi.getApi()
      .request('fetchDocuments', { contractId: secondContractPacket.getContractId(), type: documentType });

    expect(secondDriveSecondContractAfter).to.deep.equal(
      secondContractPacket.getContract().toJSON(),
    );
    expect(secondDriveSecondDocumentAfter).to.deep.equal(
      secondDocumentPacket.getDocuments()[0].toJSON(),
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
