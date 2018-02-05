const jayson = require('jayson');
const {
  getBlockchainUser,
  getBlockchainUserStateSinceHeight,
  getBlockchainUserState,
  getDapSchema,
} = require('../lib/api_methods');

const server = jayson.server({
  getBlockchainUser,
  getBlockchainUserStateSinceHeight,
  getBlockchainUserState,
  getDapSchema,
});

const bindHost = process.env.BIND_HOST || '0.0.0.0';
server.http().listen(5001, bindHost);

// break on ^C
process.on('SIGINT', () => {
  process.exit();
});
