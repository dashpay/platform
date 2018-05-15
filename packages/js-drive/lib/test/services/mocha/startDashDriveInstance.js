const startHelperWithMochaHooksFactory = require('./startHelperWithMochaHooksFactory');
const startDashDriveInstance = require('../dashDrive/startDashDriveInstance');

module.exports = startHelperWithMochaHooksFactory(startDashDriveInstance);
