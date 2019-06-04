const grpc = require('grpc');

const {
  service: healthCheckServiceDefinition,
  Implementation: HealthCheck,
} = require('grpc-health-check/health');

const {
  HealthCheckResponse: { ServingStatus: healthCheckStatuses },
} = require('grpc-health-check/v1/health_pb');

const { loadPackageDefinition } = require('@dashevo/dapi-grpc');

/**
 * @param {getTransactionsByFilterHandler} getTransactionsByFilterHandler
 * @return createServer
 */
function createServerFactory(getTransactionsByFilterHandler) {
  /**
   * @typedef createServer
   * @return {module:grpc.Server}
   */
  function createServer() {
    const server = new grpc.Server();

    // Add health check service

    const statusMap = {
      '': healthCheckStatuses.SERVING,
      'org.dash.platform.dapi.TransactionsFilterStream': healthCheckStatuses.SERVING,
    };

    server.addService(healthCheckServiceDefinition, new HealthCheck(statusMap));

    // Add TransactionsFilterStream service

    const {
      TransactionsFilterStream,
    } = loadPackageDefinition();

    server.addService(TransactionsFilterStream.service, {
      getTransactionsByFilter: getTransactionsByFilterHandler,
    });

    return server;
  }

  return createServer;
}

module.exports = createServerFactory;
