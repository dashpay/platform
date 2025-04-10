const grpc = require('@grpc/grpc-js');
const { promisify } = require('util');

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

const { URL } = require('url');
const {
  org: {
    dash: {
      platform: {
        drive: {
          v0: {
            GetProofsRequest: PBJSGetProofsRequest,
            GetProofsResponse: PBJSGetProofsResponse,
          },
        },
      },
    },
  },
} = require('./drive_pbjs');

const {
  GetProofsResponse: ProtocGetProofsResponse,
} = require('./drive_protoc');

const getDriveDefinition = require('../../../../lib/getDriveDefinition');

const DriveNodeJSClient = getDriveDefinition(0);

class DrivePromiseClient {
  /**
   * @param {string} hostname
   * @param {?Object} credentials
   * @param {?Object} options
   */
  constructor(hostname, credentials, options = {}) {
    if (credentials !== undefined) {
      throw new Error('"credentials" option is not supported yet');
    }

    const url = new URL(hostname);
    const { protocol, host: strippedHostname } = url;

    // See this issue https://github.com/nodejs/node/issues/3176
    // eslint-disable-next-line no-param-reassign
    credentials = protocol.replace(':', '') === 'https' ? grpc.credentials.createSsl() : grpc.credentials.createInsecure();

    this.client = new DriveNodeJSClient(strippedHostname, credentials, options);

    this.client.getProofs = promisify(
      this.client.getProofs.bind(this.client),
    );

    this.protocolVersion = undefined;
  }

  /**
   *
   * @param {!GetProofsRequest} request
   * @param {?Object<string, string>} metadata
   * @param {CallOptions} [options={}]
   * @returns {Promise<!GetProofsResponse>}
   */
  getProofs(request, metadata = {}, options = {}) {
    if (!isObject(metadata)) {
      throw new Error('metadata must be an object');
    }

    return this.client.getProofs(
      request,
      convertObjectToMetadata(metadata),
      {
        interceptors: [
          jsonToProtobufInterceptorFactory(
            jsonToProtobufFactory(
              ProtocGetProofsResponse,
              PBJSGetProofsResponse,
            ),
            protobufToJsonFactory(
              PBJSGetProofsRequest,
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
    this.protocolVersion = protocolVersion;
  }
}

module.exports = DrivePromiseClient;
