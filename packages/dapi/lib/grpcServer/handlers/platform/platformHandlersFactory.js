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
  ApplyStateTransitionRequest,
  GetIdentityRequest,
  GetDataContractRequest,
  GetDocumentsRequest,
  GetIdentityByFirstPublicKeyRequest,
  GetIdentityIdByFirstPublicKeyRequest,
  pbjs: {
    ApplyStateTransitionRequest: PBJSApplyStateTransitionRequest,
    ApplyStateTransitionResponse: PBJSApplyStateTransitionResponse,
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
const applyStateTransitionHandlerFactory = require(
  './applyStateTransitionHandlerFactory',
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

  // applyStateTransition
  const applyStateTransitionHandler = applyStateTransitionHandlerFactory(
    rpcClient,
    handleAbciResponseError,
  );

  const wrappedApplyStateTransition = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      ApplyStateTransitionRequest,
      PBJSApplyStateTransitionRequest,
    ),
    protobufToJsonFactory(
      PBJSApplyStateTransitionResponse,
    ),
    wrapInErrorHandler(applyStateTransitionHandler),
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
    applyStateTransition: wrappedApplyStateTransition,
    getIdentity: wrappedGetIdentity,
    getDocuments: wrappedGetDocuments,
    getDataContract: wrappedGetDataContract,
    getIdentityByFirstPublicKey: wrappedGetIdentityByFirstPublicKey,
    getIdentityIdByFirstPublicKey: wrappedGetIdentityIdByFirstPublicKey,
  };
}

module.exports = platformHandlersFactory;
