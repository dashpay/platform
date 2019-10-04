const grpc = require('grpc');

const {
  service: healthCheckServiceDefinition,
  Implementation: HealthCheck,
} = require('grpc-health-check/health');

const {
  HealthCheckResponse: { ServingStatus: healthCheckStatuses },
} = require('grpc-health-check/v1/health_pb');

const { utils: { loadPackageDefinition } } = require('@dashevo/drive-grpc');

/**
 * Creates new gRPC server
 *
 * @typedef createServer
 * @param {string} serviceName
 * @param {Object.<string, Function>} handlers
 * @return {module:grpc.Server}
 */
function createServer(serviceName, handlers) {
  const server = new grpc.Server();

  // Add 'health check' service

  const statusMap = {
    '': healthCheckStatuses.SERVING,
    [`org.dash.platform.drive.v0.${serviceName}`]: healthCheckStatuses.SERVING,
  };

  server.addService(healthCheckServiceDefinition, new HealthCheck(statusMap));

  // Add 'serviceName' service

  const {
    v0: {
      [serviceName]: Service,
    },
  } = loadPackageDefinition(serviceName);

  server.addService(Service.service, handlers);

  return server;
}

module.exports = createServer;
