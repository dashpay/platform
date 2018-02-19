const jayson = require('jayson/promise');
const rpcHandlers = require('../lib/api/rpc');

const server = jayson.server(rpcHandlers);

server.http().listen(
  process.env.API_RPC_PORT,
  process.env.API_RPC_HOST || '0.0.0.0',
);

// break on ^C
process.on('SIGINT', () => {
  process.exit();
});
