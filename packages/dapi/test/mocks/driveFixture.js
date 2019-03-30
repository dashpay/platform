/* eslint class-methods-use-this: off */
/* eslint-disable no-unused-vars */
// Unused variables represent signatures for clarity
const AbstractDriveAdapter = require('../../lib/externalApis/driveAdapter/AbstractDriveAdapter');

// Create a class, so JSDoc would work properly in our tests
class DriveFixture extends AbstractDriveAdapter {
  addSTPacket(rawStateTransition, rawSTPacket) { return Promise.resolve(); }

  fetchContract(contractId) { return Promise.resolve({}); }
}

module.exports = new DriveFixture();
