// eslint-disable-next-line import/no-extraneous-dependencies
const DashPlatformProtocol = require('@dashevo/dpp');

function getStPacketFixture() {
  const dpp = new DashPlatformProtocol();
  const contractName = Math.random().toString(36).substring(7);
  const contract = dpp.contract.create(contractName, {
    profile: {
      indices: [
        { properties: [{ $userId: 'asc' }], unique: true },
      ],
      properties: {
        avatarUrl: {
          type: 'string',
          format: 'url',
        },
        about: {
          type: 'string',
        },
      },
      required: ['avatarUrl', 'about'],
      additionalProperties: false,
    },
    contact: {
      indices: [
        { properties: [{ $userId: 'asc' }, { toUserId: 'asc' }], unique: true },
      ],
      properties: {
        toUserId: {
          type: 'string',
        },
        publicKey: {
          type: 'string',
        },
      },
      required: ['toUserId', 'publicKey'],
      additionalProperties: false,
    },
  });

  dpp.setContract(contract);
  return dpp.packet.create(contract);
}

module.exports = getStPacketFixture;
