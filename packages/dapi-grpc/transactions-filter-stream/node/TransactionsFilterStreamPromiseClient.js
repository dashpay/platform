const grpc = require('grpc');
const jsonToProtobufInterceptorFactory = require('../../src/jsonToProtobufInterceptorFactory');
const loadPackageDefinition = require('../../src/loadPackageDefinition');
const { RawTransaction } = require('./transactions_filter_stream_pb');
const isObject = require('../../src/isObject');
const convertObjectToMetadata = require('../../src/convertObjectToMetadata');

const {
  TransactionsFilterStream: TransactionsFilterStreamNodeJSClient,
} = loadPackageDefinition('TransactionsFilterStream');

const getTransactionsByFilterOptions = {
  interceptors: [jsonToProtobufInterceptorFactory(RawTransaction)],
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
   * @param {BloomFilter} bloomFilter The request proto
   * @param {?Object<string, string>} metadata User defined call metadata
   * @return {!grpc.web.ClientReadableStream<!RawTransaction>|undefined}
   *     The XHR Node Readable Stream
   */
  getTransactionsByFilter(bloomFilter, metadata = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    const message = bloomFilter.toObject();

    return this.client.getTransactionsByFilter(
      message,
      convertObjectToMetadata(metadata),
      getTransactionsByFilterOptions,
    );
  }
}

module.exports = TransactionsFilterStreamPromiseClient;
