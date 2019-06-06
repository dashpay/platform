const grpc = require('grpc');

const { InterceptingCall } = grpc;

function jsonToProtobufInterceptorFactory(MessageClass) {
  return function interceptor(options, nextCall) {
    const methods = {
      start(metadata, listener, nextStart) {
        nextStart(metadata, {
          onReceiveMessage(jsonResponse, next) {
            if (!jsonResponse) {
              return next();
            }
            const response = new MessageClass();
            Object.keys(jsonResponse).forEach((key) => {
              const setterName = `set${key[0].toUpperCase()}${key.substring(1, key.length)}`;
              response[setterName](jsonResponse[key]);
            });
            return next(response);
          },
        });
      },
    };
    return new InterceptingCall(nextCall(options), methods);
  };
}

module.exports = jsonToProtobufInterceptorFactory;
