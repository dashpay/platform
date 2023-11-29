const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');
const RPCError = require('./RPCError');
const ArgumentsValidationError = require('../errors/ArgumentsValidationError');
const DashCoreRpcError = require('../errors/DashCoreRpcError');

function isOperationalError(error) {
  return (
    (error instanceof ArgumentsValidationError)
    || (error instanceof DashCoreRpcError)
    || (error instanceof InvalidArgumentGrpcError)
  );
}

/**
 * Decorates function with an error handler
 * @param {function} command
 * @param {Logger} logger
 * @return {function(*=): Promise<T | never>}
 */
function errorHandlerDecorator(command, logger) {
  return function callCommand(args) {
    return command(args)
      .catch((e) => {
        if (e instanceof RPCError) {
          throw e;
        } else if (isOperationalError(e)) {
          throw new RPCError(-32602, e.message, e.data);
        }
        // In case if this is not a user error, print it to log and return 'Internal Error' to user
        if (logger && typeof logger.error === 'function') {
          logger.error(e);
        }
        throw new RPCError(-32603, 'Internal error');
      });
  };
}

module.exports = errorHandlerDecorator;
