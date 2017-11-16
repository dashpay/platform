const jayson = require('jayson')
const {
  getBlockchainUser,
  getBlockchainUserStateSinceHeight,
  getBlockchainUserState,
  getDapSchema
} = require('../lib/api_methods')

let server = jayson.server({
  getBlockchainUser,
  getBlockchainUserStateSinceHeight,
  getBlockchainUserState,
  getDapSchema
})

var bind_host = process.env.BIND_HOST || '0.0.0.0';
server.http().listen(5001, bind_host)

// break on ^C
process.on('SIGINT', function() {
  process.exit();
});
