const jayson = require('jayson')
let server = jayson.server({

  getBlockchainUser(args, callback) {
    callback(null, args["name"])
  },

  getBlockchainUserStateSinceHeight(args, callback) {
    callback(null, "name: " + args["name"] + ", height: " + args["height"])
  },

  getBlockchainUserState(args, callback) {
    callback(null, "name: " + args["name"] + ", dapid: " + args["dapid"])
  }

  getDapSchema(args, callback) {
    callback(null, "dapid: " + args["dapid"])
  }

})

var bind_host = process.env.BIND_HOST || '0.0.0.0';
server.http().listen(5001, bind_host)

// break on ^C
process.on('SIGINT', function() {
  process.exit();
});
