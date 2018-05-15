const startHelperWithMochaHooksFactory = require('./startHelperWithMochaHooksFactory');
const startIPFSInstance = require('../IPFS/startIPFSInstance');

module.exports = startHelperWithMochaHooksFactory(startIPFSInstance);
