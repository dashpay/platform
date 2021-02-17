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
  v0: {
    BroadcastStateTransitionRequest,
    GetIdentityRequest,
    GetDataContractRequest,
    GetDocumentsRequest,
    GetIdentitiesByPublicKeyHashesRequest,
    GetIdentityIdsByPublicKeyHashesRequest,
    WaitForStateTransitionResultRequest,
    pbjs: {
      BroadcastStateTransitionRequest: PBJSBroadcastStateTransitionRequest,
      BroadcastStateTransitionResponse: PBJSBroadcastStateTransitionResponse,
      GetIdentityRequest: PBJSGetIdentityRequest,
      GetIdentityResponse: PBJSGetIdentityResponse,
      GetDataContractRequest: PBJSGetDataContractRequest,
      GetDataContractResponse: PBJSGetDataContractResponse,
      GetDocumentsRequest: PBJSGetDocumentsRequest,
      GetDocumentsResponse: PBJSGetDocumentsResponse,
      GetIdentitiesByPublicKeyHashesResponse: PBJSGetIdentitiesByPublicKeyHashesResponse,
      GetIdentitiesByPublicKeyHashesRequest: PBJSGetIdentitiesByPublicKeyHashesRequest,
      GetIdentityIdsByPublicKeyHashesResponse: PBJSGetIdentityIdsByPublicKeyHashesResponse,
      GetIdentityIdsByPublicKeyHashesRequest: PBJSGetIdentityIdsByPublicKeyHashesRequest,
      WaitForStateTransitionResultRequest: PBJSWaitForStateTransitionResultRequest,
      WaitForStateTransitionResultResponse: PBJSWaitForStateTransitionResultResponse,
    },
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
const getIdentitiesByPublicKeyHashesHandlerFactory = require(
  './getIdentitiesByPublicKeyHashesHandlerFactory',
);
const getIdentityIdsByPublicKeyHashesHandlerFactory = require(
  './getIdentityIdsByPublicKeyHashesHandlerFactory',
);
const waitForStateTransitionResultHandlerFactory = require(
  './waitForStateTransitionResultHandlerFactory',
);

const fetchProofForStateTransitionFactory = require('../../../externalApis/drive/fetchProofForStateTransitionFactory');
const waitForTransactionToBeProvableFactory = require('../../../externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');
const waitForTransactionResult = require('../../../externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionResult');
const waitForTransactionCommitment = require('../../../externalApis/tenderdash/blockchainListener/waitForTransactionToBeProvable/waitForTransactionCommitment');

/**
 * @param {jaysonClient} rpcClient
 * @param {BlockchainListener} blockchainListener
 * @param {DriveClient} driveClient
 * @param {DashPlatformProtocol} dpp
 * @param {boolean} isProductionEnvironment
 * @returns {Object<string, function>}
 */
function platformHandlersFactory(
  rpcClient,
  blockchainListener,
  driveClient,
  dpp,
  isProductionEnvironment,
) {
  const wrapInErrorHandler = wrapInErrorHandlerFactory(log, isProductionEnvironment);

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
    driveClient, handleAbciResponseError,
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
    driveClient, handleAbciResponseError,
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
    driveClient, handleAbciResponseError,
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

  // getIdentitiesByPublicKeyHashes
  const getIdentitiesByPublicKeyHashesHandler = getIdentitiesByPublicKeyHashesHandlerFactory(
    driveClient, handleAbciResponseError,
  );

  const wrappedGetIdentitiesByPublicKeyHashes = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentitiesByPublicKeyHashesRequest,
      PBJSGetIdentitiesByPublicKeyHashesRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentitiesByPublicKeyHashesResponse,
    ),
    wrapInErrorHandler(getIdentitiesByPublicKeyHashesHandler),
  );

  // getIdentityIdsByPublicKeyHashes
  const getIdentityIdsByPublicKeyHashesHandler = getIdentityIdsByPublicKeyHashesHandlerFactory(
    driveClient, handleAbciResponseError,
  );

  const wrappedGetIdentityIdsByPublicKeyHashes = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetIdentityIdsByPublicKeyHashesRequest,
      PBJSGetIdentityIdsByPublicKeyHashesRequest,
    ),
    protobufToJsonFactory(
      PBJSGetIdentityIdsByPublicKeyHashesResponse,
    ),
    wrapInErrorHandler(getIdentityIdsByPublicKeyHashesHandler),
  );

  // waitForStateTransitionResult
  const fetchProofForStateTransition = fetchProofForStateTransitionFactory(driveClient);

  const waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
    waitForTransactionResult,
    waitForTransactionCommitment,
  );

  const waitForStateTransitionResultHandler = waitForStateTransitionResultHandlerFactory(
    fetchProofForStateTransition,
    waitForTransactionToBeProvable,
    blockchainListener,
    dpp,
  );

  const wrappedWaitForStateTransitionResult = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      WaitForStateTransitionResultRequest,
      PBJSWaitForStateTransitionResultRequest,
    ),
    protobufToJsonFactory(
      PBJSWaitForStateTransitionResultResponse,
    ),
    wrapInErrorHandler(waitForStateTransitionResultHandler),
  );

  return {
    broadcastStateTransition: wrappedBroadcastStateTransition,
    getIdentity: wrappedGetIdentity,
    getDocuments: wrappedGetDocuments,
    getDataContract: wrappedGetDataContract,
    getIdentitiesByPublicKeyHashes: wrappedGetIdentitiesByPublicKeyHashes,
    getIdentityIdsByPublicKeyHashes: wrappedGetIdentityIdsByPublicKeyHashes,
    waitForStateTransitionResult: wrappedWaitForStateTransitionResult,
  };
}

module.exports = platformHandlersFactory;
