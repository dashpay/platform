const jayson = require('jayson/promise');
const rpcHandlers = require('../lib/api/rpc');

const server = jayson.server(rpcHandlers);

const bindHost = process.env.BIND_HOST || '0.0.0.0';
server.http().listen(5001, bindHost);

// break on ^C
process.on('SIGINT', () => {
  process.exit();
});
