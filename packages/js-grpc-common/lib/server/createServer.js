const grpc = require('grpc');

const {
  service: healthCheckServiceDefinition,
  Implementation: HealthCheck,
} = require('grpc-health-check/health');

const {
  HealthCheckResponse: { ServingStatus: healthCheckStatuses },
} = require('grpc-health-check/v1/health_pb');

/**
 * Create GRPC server with a health check service
 *
 * @typedef createServer
 *
 * @param {Object<string, *>} serviceDefinition
 * @param {Object<string, Function>} handlers
 *
 * @return {module:grpc.Server}
 */
function createServer(serviceDefinition, handlers) {
  const statusMap = {
    HealthCheck: healthCheckStatuses.SERVING,
  };

  const server = new grpc.Server();
  server.addService(healthCheckServiceDefinition, new HealthCheck(statusMap));
  server.addService(serviceDefinition.service, handlers);

  return server;
}

module.exports = createServer;
