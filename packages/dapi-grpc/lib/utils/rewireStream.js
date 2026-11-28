const GrpcError = require('@dashevo/grpc-common/lib/server/error/GrpcError');

/**
 * Rewires an @improbable-eng/grpc-web stream to comply with the EventEmitter interface:
 * it reports no `error` event, and ends with the status of the call instead.
 *
 * @param {!grpc.web.ClientReadableStream} stream
 */
function rewireStream(stream) {
  const defaultOnFunction = stream.on.bind(stream);

  // eslint-disable-next-line no-param-reassign
  stream.on = (type, handler) => {
    if (type !== 'error') {
      // `data` and the other events are processed normally
      return defaultOnFunction(type, handler);
    }

    // An `error` handler hears the `end` event of a call that did not end with OK
    return stream.on('end', (payload) => {
      if (!payload) {
        return;
      }

      const { code, details, metadata } = payload;
      if (code !== 0) {
        const error = new GrpcError(code, details);
        // The metadata is assigned to the error, not passed to its constructor, which
        // would convert it to grpc-js metadata: that is not compatible with web
        error.metadata = metadata;
        handler(error);
      }
    });
  };

  // @improbable-eng/grpc-web exposes no `removeListener`, and cancelling a stream detaches
  // all of its handlers internally
  // eslint-disable-next-line no-param-reassign
  stream.removeListener = () => {};
}

module.exports = rewireStream;
