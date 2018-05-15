const startHelperWithMochaHooksFactory = require('./startHelperWithMochaHooksFactory');
const startDashCoreInstance = require('../dashCore/startDashCoreInstance');

module.exports = startHelperWithMochaHooksFactory(startDashCoreInstance);
