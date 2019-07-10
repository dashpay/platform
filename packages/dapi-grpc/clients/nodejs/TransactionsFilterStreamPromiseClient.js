const grpc = require('grpc');
const jsonToProtobufInterceptorFactory = require('../../src/interceptors/client/jsonToProtobufInterceptorFactory');
const loadPackageDefinition = require('../../src/loadPackageDefinition');

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

const isObject = require('../../src/isObject');
const convertObjectToMetadata = require('../../src/convertObjectToMetadata');

const jsonToProtobufFactory = require('../../src/converters/jsonToProtobufFactory');
const protobufToJsonFactory = require('../../src/converters/protobufToJsonFactory');

const {
  TransactionsFilterStream: TransactionsFilterStreamNodeJSClient,
} = loadPackageDefinition('TransactionsFilterStream');

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
