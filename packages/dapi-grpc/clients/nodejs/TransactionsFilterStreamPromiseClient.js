const grpc = require('grpc');

const {
  utils: {
    isObject,
    convertObjectToMetadata,
  },
  client: {
    interceptors: {
      jsonToProtobufInterceptorFactory,
    },
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
  },
} = require('@dashevo/grpc-common');

const {
  org: {
    dash: {
      platform: {
        dapi: {
          TransactionsWithProofsRequest: PBJSTransactionsWithProofsRequest,
          TransactionsWithProofsResponse: PBJSTransactionsWithProofsResponse,
        },
      },
    },
  },
} = require('./transactions_filter_stream_pbjs');

const {
  TransactionsWithProofsResponse: ProtocTransactionsWithProofsResponse,
} = require('./transactions_filter_stream_protoc');

const getTransactionsFilterStreamDefinition = require(
  '../../lib/getTransactionsFilterStreamDefinition',
);

const TransactionsFilterStreamNodeJSClient = getTransactionsFilterStreamDefinition();

const subscribeToTransactionsWithProofsOptions = {
  interceptors: [
    jsonToProtobufInterceptorFactory(
      jsonToProtobufFactory(
        ProtocTransactionsWithProofsResponse,
        PBJSTransactionsWithProofsResponse,
      ),
      protobufToJsonFactory(
        PBJSTransactionsWithProofsRequest,
      ),
    ),
  ],
};

class TransactionsFilterStreamPromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials = grpc.credentials.createInsecure(), options = {}) {
    this.client = new TransactionsFilterStreamNodeJSClient(hostname, credentials, options);
  }

  /**
   * @param {TransactionsWithProofsRequest} transactionsWithProofsRequest The request proto
   * @param {?Object<string, string>} metadata User defined call metadata
   * @return {!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>|undefined}
   *     The XHR Node Readable Stream
   */
  subscribeToTransactionsWithProofs(transactionsWithProofsRequest, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.subscribeToTransactionsWithProofs(
      transactionsWithProofsRequest,
      convertObjectToMetadata(metadata),
      subscribeToTransactionsWithProofsOptions,
    );
  }
}

module.exports = TransactionsFilterStreamPromiseClient;
