/* eslint class-methods-use-this: off */
const AbstractDashDriveAdapter = require('../../lib/api/dashDriveAdapter/AbstractDashDriveAdapter');

// Create a class, so JSDoc would work properly in our tests
class DashDriveFixture extends AbstractDashDriveAdapter {
  addSTPacket(packet) { return Promise.resolve('tsid'); }
  fetchDapContract(dapId) { return Promise.resolve({}); }
}

module.exports = new DashDriveFixture();
