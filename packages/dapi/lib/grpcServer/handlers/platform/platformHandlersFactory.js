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
  pbjs: {
    ApplyStateTransitionRequest: PBJSApplyStateTransitionRequest,
    ApplyStateTransitionResponse: PBJSApplyStateTransitionResponse,
    GetIdentityRequest: PBJSGetIdentityRequest,
    GetIdentityResponse: PBJSGetIdentityResponse,
    GetDataContractRequest: PBJSGetDataContractRequest,
    GetDataContractResponse: PBJSGetDataContractResponse,
    GetDocumentsRequest: PBJSGetDocumentsRequest,
    GetDocumentsResponse: PBJSGetDocumentsResponse,
  },
} = require('@dashevo/dapi-grpc');

const log = require('../../../log');

const handleAbciResponse = require('../handleAbciResponse');

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

/**
 * @param {jaysonClient} rpcClient
 * @param {DriveAdapter} driveAPI
 * @param {DashPlatformProtocol} dpp
 * @returns {Object<string, function>}
 */
function platformHandlersFactory(rpcClient, driveAPI, dpp) {
  const wrapInErrorHandler = wrapInErrorHandlerFactory(log);

  // applyStateTransition
  const applyStateTransitionHandler = applyStateTransitionHandlerFactory(
    rpcClient,
    handleAbciResponse,
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
  const getIdentityHandler = getIdentityHandlerFactory(rpcClient, handleAbciResponse);

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
  const getDocumentsHandler = getDocumentsHandlerFactory(driveAPI, dpp);

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
  const getDataContractHandler = getDataContractHandlerFactory(driveAPI, dpp);

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

  return {
    applyStateTransition: wrappedApplyStateTransition,
    getIdentity: wrappedGetIdentity,
    getDocuments: wrappedGetDocuments,
    getDataContract: wrappedGetDataContract,
  };
}

module.exports = platformHandlersFactory;
