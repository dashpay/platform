var jayson = require('jayson')
var server = jayson.server({
  getBlockchainUser(args, callback) {
    callback(null, args["name"])
  },

  getBlockchainUserState(args, callback) {
    callback(null, "name: " + args["name"] + ", dapid: " + args["dapid"])
  }

})

var bind_host = process.env.BIND_HOST || '0.0.0.0';
server.http().listen(5001, bind_host)

// break on ^C
process.on('SIGINT', function() {
  process.exit();
});
