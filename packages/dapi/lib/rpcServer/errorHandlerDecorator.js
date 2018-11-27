const RPCError = require('./RPCError');

/**
 * Decorates function with an error handler
 * @param {function} command
 * @return {function(*=): Promise<T | never>}
 */
function errorHandlerDecorator(command) {
  return function callCommand(args) {
    return command(args).catch((e) => {
      if (e instanceof RPCError) {
        throw e;
      }
      throw new RPCError(-32602, e.message);
    });
  };
}

module.exports = errorHandlerDecorator;
