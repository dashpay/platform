var jayson = require('jayson')
var server = jayson.server({
  add: function(args, callback) {
    callback(null, args[0] + args[1])
  },

  getBlockchainUser: function(args, callback) {
    callback(null, args["name"])
  }
})

var bind_host = process.env.BIND_HOST || '0.0.0.0';
server.http().listen(5001, bind_host)

// break on ^C
process.on('SIGINT', function() {
  process.exit();
});
