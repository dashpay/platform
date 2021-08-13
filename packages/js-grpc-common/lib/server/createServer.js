const grpc = require('@grpc/grpc-js');

/**
 * Create GRPC server
 *
 * @typedef createServer
 *
 * @param {Object<string, *>} serviceDefinition
 * @param {Object<string, Function>} handlers
 *
 * @return {module:grpc.Server}
 */
function createServer(serviceDefinition, handlers) {
  const server = new grpc.Server();
  server.addService(serviceDefinition.service, handlers);

  return server;
}

module.exports = createServer;
