/**
 * Server-side JSON -> protobuf -> JSON handler wrapper (factory)
 *
 * @param {jsonToProtobuf} jsonToProtobuf
 * @param {protobufToJson} protobufToJson
 * @param {function(grpc.Call, function(Error|null, jspb.Message|null))} rpcMethod
 *
 * @returns {wrappedMethodHandler}
 */
function jsonToProtobufHandlerWrapper(jsonToProtobuf, protobufToJson, rpcMethod) {
  /**
   * Decorate `request` and `write`
   *
   * @param {grpc.Call} call
   *
   * @returns {grpc.Call}
   */
  function decorateCall(call) {
    return new Proxy(call, {
      get(target, propKey) {
        if (propKey === 'request') {
          return jsonToProtobuf(target[propKey]);
        }

        if (propKey === 'write') {
          return (message, flags, writeCallback) => {
            let convertedMessage = null;
            if (message) {
              convertedMessage = protobufToJson(message);
            }
            return call.write(convertedMessage, flags, writeCallback);
          };
        }

        return target[propKey];
      },
    });
  }

  /**
   * Server-side JSON -> protobuf -> JSON handler wrapper
   *
   * @typedef wrappedMethodHandler
   *
   * @param {grpc.Call} call
   * @param {function(Error|null, jspb.Message|null)} callback
   *
   * @returns {*}
   */
  function methodHandler(call, callback = undefined) {
    const proxyCall = decorateCall(call);

    let interceptedCallback;
    if (callback) {
      interceptedCallback = (err, message) => {
        let convertedMessage = null;
        if (message) {
          convertedMessage = protobufToJson(message);
        }
        callback(err, convertedMessage);
      };
    }

    return rpcMethod(proxyCall, interceptedCallback);
  }

  return methodHandler;
}

module.exports = jsonToProtobufHandlerWrapper;
