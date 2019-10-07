const jsonToProtobufFactory = require('./lib/client/converters/jsonToProtobufFactory');
const protobufToJsonFactory = require('./lib/client/converters/protobufToJsonFactory');
const jsonToProtobufInterceptorFactory = require(
  './lib/client/interceptors/jsonToProtobufInterceptorFactory',
);

const createServer = require('./lib/server/createServer');
const jsonToProtobufHandlerWrapper = require(
  './lib/server/jsonToProtobufHandlerWrapper',
);
const AcknowledgingWritable = require('./lib/server/stream/AcknowledgingWritable');
const wrapInErrorHandlerFactory = require('./lib/server/error/wrapInErrorHandlerFactory');

const isObject = require('./lib/utils/isObject');

const convertObjectToMetadata = require('./lib/convertObjectToMetadata');
const loadPackageDefinition = require('./lib/loadPackageDefinition');

module.exports = {
  loadPackageDefinition,
  convertObjectToMetadata,
  client: {
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
    interceptors: {
      jsonToProtobufInterceptorFactory,
    },
  },
  server: {
    createServer,
    jsonToProtobufHandlerWrapper,
    stream: {
      AcknowledgingWritable,
    },
    error: {
      wrapInErrorHandlerFactory,
    },
  },
  utils: {
    isObject,
  },
};
