var jayson = require('jayson')
var server = jayson.server({
  add: function(args, callback) {
    callback(null, args[0] + args[1])
  },

  getBlockchainUser: function(args, callback) {
    callback(null, args[0] + args[1])
  }
})

server.http().listen(3000, '127.0.0.1')
