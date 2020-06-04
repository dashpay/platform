const grpc = require('grpc');

const {
  convertObjectToMetadata,
  utils: {
    isObject,
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
          v0: {
            TransactionsWithProofsRequest: PBJSTransactionsWithProofsRequest,
            TransactionsWithProofsResponse: PBJSTransactionsWithProofsResponse,
          },
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
const stripHostname = require('../../lib/utils/stripHostname');

const TransactionsFilterStreamNodeJSClient = getTransactionsFilterStreamDefinition();

class TransactionsFilterStreamPromiseClient {
  /**
   * @param {string} hostname
   * @param {string} version
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials = grpc.credentials.createInsecure(), options = {}) {
    const strippedHostname = stripHostname(hostname);

    this.client = new TransactionsFilterStreamNodeJSClient(strippedHostname, credentials, options);

    this.protocolVersion = undefined;
  }

  /**
   * @param {TransactionsWithProofsRequest} transactionsWithProofsRequest The request proto
   * @param {?Object<string, string>} metadata User defined call metadata
   * @param {CallOptions} [options={}]
   * @return {!grpc.web.ClientReadableStream<!TransactionsWithProofsResponse>|undefined}
   *     The XHR Node Readable Stream
   */
  subscribeToTransactionsWithProofs(transactionsWithProofsRequest, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.subscribeToTransactionsWithProofs(
      transactionsWithProofsRequest,
      convertObjectToMetadata(metadata),
      {
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
        ...options,
      },
    );
  }

  /**
   * @param {string} protocolVersion
   */
  setProtocolVersion(protocolVersion) {
    this.setProtocolVersion = protocolVersion;
  }
}

module.exports = TransactionsFilterStreamPromiseClient;
