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
    GetConsensusParamsRequest,
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
      GetConsensusParamsRequest: PBJSGetConsensusParamsRequest,
      GetConsensusParamsResponse: PBJSGetConsensusParamsResponse,
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
const getConsensusParamsHandlerFactory = require(
  './getConsensusParamsHandlerFactory',
);

const fetchProofForStateTransitionFactory = require('../../../externalApis/drive/fetchProofForStateTransitionFactory');
const waitForTransactionToBeProvableFactory = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionToBeProvableFactory');
const waitForTransactionResult = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/waitForTransactionResult');
const waitForHeightFactory = require('../../../externalApis/tenderdash/waitForHeightFactory');
const getExistingTransactionResultFactory = require('../../../externalApis/tenderdash/waitForTransactionToBeProvable/getExistingTransactionResult');
const getConsensusParamsFactory = require('../../../externalApis/tenderdash/getConsensusParamsFactory');

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

  const getExistingTransactionResult = getExistingTransactionResultFactory(
    rpcClient,
  );

  const waitForHeight = waitForHeightFactory(blockchainListener);

  const waitForTransactionToBeProvable = waitForTransactionToBeProvableFactory(
    waitForTransactionResult,
    getExistingTransactionResult,
    waitForHeight,
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

  // get Consensus Params
  const getConsensusParams = getConsensusParamsFactory(rpcClient);
  const getConsensusParamsHandler = getConsensusParamsHandlerFactory(getConsensusParams);

  const wrappedGetConsensusParams = jsonToProtobufHandlerWrapper(
    jsonToProtobufFactory(
      GetConsensusParamsRequest,
      PBJSGetConsensusParamsRequest,
    ),
    protobufToJsonFactory(
      PBJSGetConsensusParamsResponse,
    ),
    wrapInErrorHandler(getConsensusParamsHandler),
  );

  return {
    broadcastStateTransition: wrappedBroadcastStateTransition,
    getIdentity: wrappedGetIdentity,
    getDocuments: wrappedGetDocuments,
    getDataContract: wrappedGetDataContract,
    getIdentitiesByPublicKeyHashes: wrappedGetIdentitiesByPublicKeyHashes,
    getIdentityIdsByPublicKeyHashes: wrappedGetIdentityIdsByPublicKeyHashes,
    waitForStateTransitionResult: wrappedWaitForStateTransitionResult,
    getConsensusParams: wrappedGetConsensusParams,
  };
}

module.exports = platformHandlersFactory;
