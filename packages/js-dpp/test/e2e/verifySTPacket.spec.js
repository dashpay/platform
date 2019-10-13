const { startDrive } = require('@dashevo/dp-services-ctl');

const DashPlatformProtocol = require('../../lib/DashPlatformProtocol');

const registerUser = require('../../lib/test/e2e/registerUser');
const sendSTPacket = require('../../lib/test/e2e/sendSTPacket');
const createStateTransition = require('../../lib/test/e2e/createStateTransition');
const isDriveSynced = require('../../lib/test/e2e/isDriveSynced');

describe.skip('verifySTPacket', function describe() {
  this.timeout(90000);

  let dpp;
  let drive;

  before(async () => {
    dpp = new DashPlatformProtocol();

    drive = await startDrive();

    // Activate Special Transactions
    await drive.dashCore.getApi().generate(1000);
  });

  it('should verify DP object uniqueness by indices by submitting correct queries to Drive', async () => {
    // Register a user
    const user = await registerUser(
      'simpleBlockchainUser',
      drive.dashCore.getApi(),
    );

    // Create the data contract
    const dataContract = dpp.dataContract.create('IndexedContract', {
      profile: {
        indices: [
          {
            properties: [
              { $userId: 'asc' },
              { firstName: 'desc' },
            ],
            unique: true,
          },
          {
            properties: [
              { $userId: 'asc' },
              { email: 'asc' },
            ],
            unique: true,
          },
        ],
        properties: {
          firstName: {
            type: 'string',
          },
          email: {
            type: 'string',
          },
        },
        required: ['firstName', 'email'],
        additionalProperties: false,
      },
    });

    dpp.setDataContract(dataContract);

    const contractPacket = dpp.packet.create(dataContract);

    const contractTransaction = createStateTransition(
      user,
      contractPacket,
    );

    const contractTsId = await sendSTPacket(
      contractPacket,
      contractTransaction,
      drive.driveApi.getApi(),
      drive.dashCore.getApi(),
    );

    await isDriveSynced(drive.driveApi.getApi());

    // Create first user object
    dpp.setUserId(user.getId());

    const firstUserDocument = dpp.document.create('profile', {
      firstName: 'William',
      email: 'w.birkin@umbrella.co',
    });

    firstUserDocument.removeMetadata();

    const firstUserPacket = dpp.packet.create([firstUserDocument]);
    const firstUserTransaction = createStateTransition(
      user,
      firstUserPacket,
      contractTsId,
    );

    const firstUserTsId = await sendSTPacket(
      firstUserPacket,
      firstUserTransaction,
      drive.driveApi.getApi(),
      drive.dashCore.getApi(),
    );

    await isDriveSynced(drive.driveApi.getApi());

    // Create second user object
    const secondUserDocument = dpp.document.create('profile', {
      firstName: 'Annette',
      email: 'a.birkin@umbrella.co',
    });

    secondUserDocument.removeMetadata();

    const secondUserPacket = dpp.packet.create([secondUserDocument]);
    const secondUserTransaction = createStateTransition(
      user,
      secondUserPacket,
      firstUserTsId,
    );

    const secondUserTsId = await sendSTPacket(
      secondUserPacket,
      secondUserTransaction,
      drive.driveApi.getApi(),
      drive.dashCore.getApi(),
    );

    await isDriveSynced(drive.driveApi.getApi());

    // Create third user object violating unique indices
    const thirdUserDocument = dpp.document.create('profile', {
      firstName: 'Leon',
      email: 'a.birkin@umbrella.co',
    });

    thirdUserDocument.removeMetadata();

    const thirdUserPacket = dpp.packet.create([thirdUserDocument]);
    const thirdUserTransaction = createStateTransition(
      user,
      thirdUserPacket,
      secondUserTsId,
    );

    try {
      await sendSTPacket(
        thirdUserPacket,
        thirdUserTransaction,
        drive.driveApi.getApi(),
        drive.dashCore.getApi(),
      );

      expect.fail('Duplicate object was successfully sent');
    } catch (e) {
      const error = e.originalError;

      expect(error).to.have.property('data');
      expect(error.data[0].name).to.equal('DuplicateDocumentError');
      expect(error.data[0].document).to.deep.equal(thirdUserDocument.toJSON());
      expect(error.data[0].indexDefinition).to.deep.equal({
        unique: true,
        properties: [
          { $userId: 'asc' },
          { email: 'asc' },
        ],
      });
    }
  });

  after(async () => {
    await drive.remove();
  });
});
