const grpc = require('grpc');

const {
  service: healthCheckServiceDefinition,
  Implementation: HealthCheck,
} = require('grpc-health-check/health');

const {
  HealthCheckResponse: { ServingStatus: healthCheckStatuses },
} = require('grpc-health-check/v1/health_pb');

const loadPackageDefinition = require('../loadPackageDefinition');

/**
 * Create GRPC server with a health check service
 *
 * @typedef createServer
 *
 * @param {string} serviceName - Full service name path, including namespace
 * @param {string} protoPath
 * @param {Object.<string, Function>} handlers
 *
 * @return {module:grpc.Server}
 */
function createServer(serviceName, protoPath, handlers) {
  const statusMap = {
    '': healthCheckStatuses.SERVING,
    [serviceName]: healthCheckStatuses.SERVING,
  };

  const Service = loadPackageDefinition(
    protoPath,
    serviceName,
  );

  const server = new grpc.Server();
  server.addService(healthCheckServiceDefinition, new HealthCheck(statusMap));
  server.addService(Service.service, handlers);

  return server;
}

module.exports = createServer;
