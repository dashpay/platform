const {
  client: {
    converters: {
      jsonToProtobufFactory,
      protobufToJsonFactory,
    },
  },
  server: {
    jsonToProtobufHandlerWrapper,
    error: {
      wrapInErrorHandlerFactory,
    },
  },
} = require('@dashevo/grpc-common');

const {
  BroadcastStateTransitionRequest,
  GetIdentityRequest,
  GetDataContractRequest,
  GetDocumentsRequest,
  GetIdentityByFirstPublicKeyRequest,
  GetIdentityIdByFirstPublicKeyRequest,
  pbjs: {
    BroadcastStateTransitionRequest: PBJSBroadcastStateTransitionRequest,
    BroadcastStateTransitionResponse: PBJSBroadcastStateTransitionResponse,
    GetIdentityRequest: PBJSGetIdentityRequest,
    GetIdentityResponse: PBJSGetIdentityResponse,
    GetDataContractRequest: PBJSGetDataContractRequest,
    GetDataContractResponse: PBJSGetDataContractResponse,
    GetDocumentsRequest: PBJSGetDocumentsRequest,
    GetDocumentsResponse: PBJSGetDocumentsResponse,
    GetIdentityByFirstPublicKeyResponse: PBJSGetIdentityByFirstPublicKeyResponse,
    GetIdentityByFirstPublicKeyRequest: PBJSGetIdentityByFirstPublicKeyRequest,
    GetIdentityIdByFirstPublicKeyResponse: PBJSGetIdentityIdByFirstPublicKeyResponse,
    GetIdentityIdByFirstPublicKeyRequest: PBJSGetIdentityIdByFirstPublicKeyRequest,
  },
} = require('@dashevo/dapi-grpc');

const log = require('../../../log');

const handleAbciResponseError = require('../handleAbciResponseError');

const getIdentityHandlerFactory = require(
  './getIdentityHandlerFactory',
);
const broadcastStateTransitionHandlerFactory = require(
  './broadcastStateTransitionHandlerFactory',
);
const getDocumentsHandlerFactory = require(
  './getDocumentsHandlerFactory',
);
const getDataContractHandlerFactory = require(
  './getDataContractHandlerFactory',
);
const getIdentityByFirstPublicKeyHandlerFactory = require(
  './getIdentityByFirstPublicKeyHandlerFactory',
);
const getIdentityIdByFirstPublicKeyHandlerFactory = require(
  './getIdentityIdByFirstPublicKeyHandlerFactory',
);

/**
 * @param {jaysonClient} rpcClient
 * @param {DriveStateRepository} driveStateRepository
 * @returns {Object<string, function>}
 */
function platformHandlersFactory(rpcClient, driveStateRepository) {
  const wrapInErrorHandler = wrapInErrorHandlerFactory(log);

  // broadcastStateTransition
  const broadcastStateTransitionHandler = broadcastStateTransitionHandlerFactory(
    rpcClient,
    handleAbciResponseError,
  );

  const wrappedBroadcastStateTransition = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      BroadcastStateTransitionRequest,
      PBJSBroadcastStateTransitionRequest,
    ),
    protobufToJsonFactory(
      PBJSBroadcastStateTransitionResponse,
    ),
    wrapInErrorHandler(broadcastStateTransitionHandler),
  );

  // getIdentity
  const getIdentityHandler = getIdentityHandlerFactory(
    driveStateRepository, handleAbciResponseError,
  );

  const wrappedGetIdentity = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityRequest,
      PBJSGetIdentityRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityResponse,
    ),
    wrapInErrorHandler(getIdentityHandler),
  );

  // getDocuments
  const getDocumentsHandler = getDocumentsHandlerFactory(
    driveStateRepository, handleAbciResponseError,
  );

  const wrappedGetDocuments = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetDocumentsRequest,
      PBJSGetDocumentsRequest,
    ),
    protobufToJsonFactory(
      PBJSGetDocumentsResponse,
    ),
    wrapInErrorHandler(getDocumentsHandler),
  );

  // getDataContract
  const getDataContractHandler = getDataContractHandlerFactory(
    driveStateRepository, handleAbciResponseError,
  );

  const wrappedGetDataContract = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetDataContractRequest,
      PBJSGetDataContractRequest,
    ),
    protobufToJsonFactory(
      PBJSGetDataContractResponse,
    ),
    wrapInErrorHandler(getDataContractHandler),
  );

  // getIdentityByFirstPublicKey
  const getIdentityByFirstPublicKeyHandler = getIdentityByFirstPublicKeyHandlerFactory(
    driveStateRepository, handleAbciResponseError,
  );

  const wrappedGetIdentityByFirstPublicKey = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityByFirstPublicKeyRequest,
      PBJSGetIdentityByFirstPublicKeyRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityByFirstPublicKeyResponse,
    ),
    wrapInErrorHandler(getIdentityByFirstPublicKeyHandler),
  );

  // getIdentityIdByFirstPublicKey
  const getIdentityIdByFirstPublicKeyHandler = getIdentityIdByFirstPublicKeyHandlerFactory(
    driveStateRepository, handleAbciResponseError,
  );

  const wrappedGetIdentityIdByFirstPublicKey = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityIdByFirstPublicKeyRequest,
      PBJSGetIdentityIdByFirstPublicKeyRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityIdByFirstPublicKeyResponse,
    ),
    wrapInErrorHandler(getIdentityIdByFirstPublicKeyHandler),
  );

  return {
    broadcastStateTransition: wrappedBroadcastStateTransition,
    getIdentity: wrappedGetIdentity,
    getDocuments: wrappedGetDocuments,
    getDataContract: wrappedGetDataContract,
    getIdentityByFirstPublicKey: wrappedGetIdentityByFirstPublicKey,
    getIdentityIdByFirstPublicKey: wrappedGetIdentityIdByFirstPublicKey,
  };
}

module.exports = platformHandlersFactory;
