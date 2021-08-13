const grpc = require('@grpc/grpc-js');

const { InterceptingCall } = grpc;

/**
 * Client-side JSON -> protobuf -> JSON interceptor (factory)
 *
 * @param {jsonToProtobuf} jsonToProtobuf
 * @param {protobufToJson} protobufToJson
 *
 * @returns {conversionInterceptor}
 */
function jsonToProtobufInterceptorFactory(jsonToProtobuf, protobufToJson) {
  /**
   * Client-side JSON -> protobuf -> JSON interceptor
   *
   * @param {Object} options
   * @param {module:grpc.InterceptingCall} nextCall
   *
   * @returns {module:grpc.InterceptingCall}
   */
  function conversionInterceptor(options, nextCall) {
    const methods = {
      start(metadata, listener, nextStart) {
        nextStart(metadata, {
          onReceiveMessage(jsonResponse, next) {
            if (!jsonResponse) {
              return next();
            }

            return next(jsonToProtobuf(jsonResponse));
          },
        });
      },
      sendMessage(message, next) {
        next(protobufToJson(message));
      },
    };
    return new InterceptingCall(nextCall(options), methods);
  }

  return conversionInterceptor;
}

module.exports = jsonToProtobufInterceptorFactory;
